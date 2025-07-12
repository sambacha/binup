use crate::cli::CiContext;
use crate::constants::{
    RETRY_EXPONENTIAL_BASE, RETRY_INITIAL_DELAY_MS, RETRY_MAX_ATTEMPTS, RETRY_MAX_DELAY_MS,
};
use anyhow::{anyhow, bail, Context, Result};
use console::style;
#[cfg(not(target_os = "freebsd"))]
use flate2::read::GzDecoder;
use indicatif::{ProgressBar, ProgressStyle};
use std::fs::File;
#[cfg(not(target_os = "freebsd"))]
use std::path::Component::{self, Normal};
use std::{
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    thread,
    time::Duration,
};
#[cfg(not(target_os = "freebsd"))]
use tar::Archive;
use zip::ZipArchive;

/// Configuration for download retry logic
struct RetryConfig {
    max_attempts: u32,
    initial_delay_ms: u64,
    max_delay_ms: u64,
    exponential_base: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: RETRY_MAX_ATTEMPTS,
            initial_delay_ms: RETRY_INITIAL_DELAY_MS,
            max_delay_ms: RETRY_MAX_DELAY_MS,
            exponential_base: RETRY_EXPONENTIAL_BASE,
        }
    }
}

/// Execute a download operation with retry logic
fn download_with_retry<F, T>(operation: F, url: &str) -> Result<T>
where
    F: Fn() -> Result<T>,
{
    let config = RetryConfig::default();
    let mut last_error = None;

    for attempt in 0..config.max_attempts {
        match operation() {
            Ok(result) => return Ok(result),
            Err(e) => {
                last_error = Some(e);

                if attempt < config.max_attempts - 1 {
                    let delay = calculate_retry_delay(attempt, &config);
                    eprintln!(
                        "  {} downloading from {} (attempt {}/{}). Retrying in {} seconds...",
                        style("Failed").red(),
                        url,
                        attempt + 1,
                        config.max_attempts,
                        delay.as_secs_f64()
                    );
                    thread::sleep(delay);
                }
            }
        }
    }

    Err(last_error
        .unwrap_or_else(|| anyhow!("Download failed after {} attempts", config.max_attempts)))
}

fn calculate_retry_delay(attempt: u32, config: &RetryConfig) -> Duration {
    let exponential_delay =
        config.initial_delay_ms as f64 * config.exponential_base.powi(attempt as i32);
    let capped_delay = exponential_delay.min(config.max_delay_ms as f64) as u64;
    Duration::from_millis(capped_delay)
}

#[cfg(not(target_os = "freebsd"))]
pub(crate) fn unpack_sans_parent<R, P>(src: R, dst: P, levels_to_skip: usize) -> Result<()>
where
    R: Read,
    P: AsRef<Path>,
{
    let tar = GzDecoder::new(src);
    let mut archive = Archive::new(tar);
    for entry in archive.entries()? {
        let mut entry = entry?;
        let entry_path = entry.path()?;
        let path: PathBuf = {
            let mut components = Vec::new();
            for component in entry_path.components().skip(levels_to_skip) {
                match component {
                    Component::Normal(name) => components.push(name),
                    Component::ParentDir | Component::CurDir => {
                        return Err(anyhow::anyhow!(
                            "Archive contains unsafe path component: {}",
                            entry_path.display()
                        ));
                    }
                    _ => {
                        return Err(anyhow::anyhow!(
                            "Archive contains non-standard path component: {}",
                            entry_path.display()
                        ));
                    }
                }
            }
            components.iter().collect()
        };
        entry.unpack(dst.as_ref().join(path))?;
    }
    Ok(())
}

#[cfg(target_os = "freebsd")]
pub(crate) fn unpack_sans_parent<R, P>(mut src: R, dst: P, levels_to_skip: usize) -> Result<()>
where
    R: Read,
    P: AsRef<Path>,
{
    std::fs::create_dir_all(dst.as_ref())?;
    let mut tar = std::process::Command::new("tar")
        .arg("-C")
        .arg(dst.as_ref())
        .arg("-x")
        .arg("-z")
        .arg(format!("--strip-components={}", levels_to_skip))
        .arg("-f")
        .arg("-")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .spawn()
        .expect("Failed to spawn `tar` process");
    let mut stdin = tar
        .stdin
        .take()
        .expect("Failed to get stdin for `tar` process");
    std::io::copy(&mut src, &mut stdin)?;
    Ok(())
}

#[cfg(not(windows))]
pub fn download_extract_sans_parent(
    url: &str,
    target_path: &Path,
    levels_to_skip: usize,
) -> Result<String> {
    download_with_retry(
        || {
            log::debug!("Downloading from url `{}`.", url);
            let response = reqwest::blocking::get(url)
                .with_context(|| format!("Failed to download from url `{}`.", url))?;

            let content_length = response.content_length();

            // Check CI context to determine if progress bar should be shown
            let ci_context = CiContext::detect();
            let pb = if ci_context.should_show_progress() {
                let pb = match content_length {
                    Some(content_length) => ProgressBar::new(content_length),
                    None => ProgressBar::new_spinner(),
                };

                pb.set_prefix(style("  Downloading:").cyan().bold().to_string());
                pb.set_style(
                    ProgressStyle::default_bar()
                        .template("{prefix} [{bar}] {bytes}/{total_bytes} eta: {eta}")
                        .unwrap()
                        .progress_chars("=> "),
                );
                Some(pb)
            } else {
                None
            };

            let last_modified = match response
        .headers()
        .get("etag") {
            Some(etag) => Ok(etag.to_str().unwrap_or_default().to_string()),
            None => Err(anyhow!(format!("Failed to get etag from `{}`.\n\
                This is likely due to requesting a pull request that does not have a cached build available. You may have to build locally.", url))),
        }?;

            let response_reader: Box<dyn std::io::Read> = if let Some(ref pb) = pb {
                Box::new(pb.wrap_read(response))
            } else {
                Box::new(response)
            };

            unpack_sans_parent(response_reader, target_path, levels_to_skip).with_context(
                || format!("Failed to extract downloaded file from url `{}`.", url),
            )?;

            Ok(last_modified)
        },
        url,
    )
}

// Helper function to unpack ZIP archives
// Note: strip_components for ZIP might be less straightforward than for tar,
// as ZIP files don't necessarily have a single root directory.
// This implementation mimics the tar behavior by skipping path components.
#[cfg(not(target_os = "freebsd"))] // Assuming zip crate works on non-FreeBSD, adjust if needed
fn unpack_zip<R, P>(src: R, dst: P, levels_to_skip: usize) -> Result<()>
// Removed mut from src
where
    R: Read + std::io::Seek, // ZipArchive needs Seek
    P: AsRef<Path>,
{
    let mut archive = ZipArchive::new(src)?;
    std::fs::create_dir_all(dst.as_ref())?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let outpath_rel: PathBuf = file
            .enclosed_name()
            .ok_or_else(|| anyhow!("Zip entry has invalid path: {}", file.name()))?
            .components()
            .skip(levels_to_skip)
            .filter(|c| matches!(c, Normal(_)))
            .collect();

        if outpath_rel.as_os_str().is_empty() {
            continue; // Skip if path becomes empty after stripping
        }

        let outpath_abs = dst.as_ref().join(&outpath_rel);

        if file.name().ends_with('/') {
            std::fs::create_dir_all(&outpath_abs)?;
        } else {
            if let Some(p) = outpath_abs.parent() {
                if !p.exists() {
                    std::fs::create_dir_all(p)?;
                }
            }
            let mut outfile = File::create(&outpath_abs)?;
            std::io::copy(&mut file, &mut outfile)?;
        }

        // Restore permissions on Unix-like systems
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Some(mode) = file.unix_mode() {
                std::fs::set_permissions(&outpath_abs, std::fs::Permissions::from_mode(mode))?;
            }
        }
    }
    Ok(())
}

// Placeholder for FreeBSD zip extraction, could use `unzip` command if available
#[cfg(target_os = "freebsd")]
fn unpack_zip<R, P>(mut src: R, dst: P, levels_to_skip: usize) -> Result<()>
where
    R: Read + std::io::Seek,
    P: AsRef<Path>,
{
    std::fs::create_dir_all(dst.as_ref()).with_context(|| {
        format!(
            "Failed to create destination directory: {}",
            dst.as_ref().display()
        )
    })?;

    // Create a temporary file to store the zip content
    let mut temp_zip_file = tempfile::Builder::new()
        .suffix(".zip")
        .tempfile()
        .with_context(|| "Failed to create temporary file for ZIP extraction")?;

    std::io::copy(&mut src, temp_zip_file.as_file_mut())
        .with_context(|| "Failed to copy ZIP data to temporary file")?;

    // Use the unzip command to extract
    let mut cmd = std::process::Command::new("unzip");
    cmd.arg("-q") // Quiet operation
        .arg("-o") // Overwrite files without prompting
        .arg("-d")
        .arg(dst.as_ref())
        .arg(temp_zip_file.path());

    let output = cmd.output()
        .with_context(|| "Failed to execute 'unzip' command. Please ensure 'unzip' is installed on your FreeBSD system.")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "unzip command failed with status {}: {}",
            output.status,
            stderr
        );
    }

    // Handle strip_components manually by moving files up the directory tree
    if levels_to_skip > 0 {
        // Find the top-level directories to skip
        let mut entries = std::fs::read_dir(dst.as_ref()).with_context(|| {
            format!(
                "Failed to read extracted directory: {}",
                dst.as_ref().display()
            )
        })?;

        if let Some(entry) = entries.next() {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                let mut current_path = entry.path();

                // Navigate down the directory tree by levels_to_skip
                for _ in 1..levels_to_skip {
                    let mut sub_entries = std::fs::read_dir(&current_path)?;
                    if let Some(sub_entry) = sub_entries.next() {
                        let sub_entry = sub_entry?;
                        if sub_entry.file_type()?.is_dir() {
                            current_path = sub_entry.path();
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                }

                // Move contents of current_path to dst
                for entry in std::fs::read_dir(&current_path)? {
                    let entry = entry?;
                    let dest_path = dst.as_ref().join(entry.file_name());
                    std::fs::rename(entry.path(), dest_path).with_context(|| {
                        format!(
                            "Failed to move {} during strip_components",
                            entry.path().display()
                        )
                    })?;
                }

                // Clean up the now-empty nested directories
                std::fs::remove_dir_all(dst.as_ref().join(entry.file_name()))?;
            }
        }
    }

    Ok(())
}

#[cfg(not(windows))]
pub fn download_extract_zip(
    url: &str,
    target_path: &Path,
    levels_to_skip: usize,
) -> Result<String> {
    download_with_retry(
        || {
            log::debug!("Downloading ZIP from url `{}`.", url);
            let response = reqwest::blocking::get(url)
                .with_context(|| format!("Failed to download ZIP from url `{}`.", url))?;

            let content_length = response.content_length();
            // Check CI context to determine if progress bar should be shown
            let ci_context = CiContext::detect();
            let pb = if ci_context.should_show_progress() {
                let pb = match content_length {
                    Some(len) => ProgressBar::new(len),
                    None => ProgressBar::new_spinner(),
                };
                pb.set_prefix(style("  Downloading ZIP:").cyan().bold().to_string());
                pb.set_style(
                    ProgressStyle::default_bar()
                        .template("{prefix} [{bar}] {bytes}/{total_bytes} eta: {eta}")
                        .unwrap()
                        .progress_chars("=> "),
                );
                Some(pb)
            } else {
                None
            };

            let last_modified = response
                .headers()
                .get("etag")
                .map(|etag| etag.to_str().unwrap_or_default().to_string())
                .unwrap_or_else(|| "unknown_etag".to_string()); // Provide a default if ETag is missing

            // ZipArchive::new requires Read + Seek. reqwest::Response is not Seek.
            // We need to download to a temporary file or in-memory buffer first.
            // For potentially large files, a temporary file is safer.
            let mut temp_file = tempfile::Builder::new().suffix(".zip").tempfile()?;
            let mut response_reader: Box<dyn std::io::Read> = if let Some(ref pb) = pb {
                Box::new(pb.wrap_read(response))
            } else {
                Box::new(response)
            };
            std::io::copy(&mut response_reader, &mut temp_file)?; // temp_file is a NamedTempFile

            // To use seek, we need to operate on the File trait object it derefs to,
            // or get the File explicitly.
            // Since NamedTempFile derefs to File, this should work if Seek is in scope.
            temp_file.seek(SeekFrom::Start(0))?; // Rewind the temp file before passing to ZipArchive

            unpack_zip(temp_file, target_path, levels_to_skip) // unpack_zip expects R: Read + Seek
                .with_context(|| {
                    format!("Failed to extract downloaded ZIP file from url `{}`.", url)
                })?;

            Ok(last_modified)
        },
        url,
    )
}

#[cfg(not(windows))]
pub fn download_binary(url: &str, target_file_path: &Path) -> Result<String> {
    download_with_retry(
        || {
            log::debug!(
                "Downloading binary from url `{}` to `{}`.",
                url,
                target_file_path.display()
            );
            let response = reqwest::blocking::get(url)
                .with_context(|| format!("Failed to download binary from url `{}`.", url))?;

            if !response.status().is_success() {
                bail!(
                    "Failed to download binary from {}: HTTP status {}",
                    url,
                    response.status()
                );
            }

            let content_length = response.content_length();

            // Check CI context to determine if progress bar should be shown
            let ci_context = CiContext::detect();
            let pb = if ci_context.should_show_progress() {
                let pb = match content_length {
                    Some(len) => ProgressBar::new(len),
                    None => ProgressBar::new_spinner(),
                };
                pb.set_prefix(style("  Downloading Binary:").cyan().bold().to_string());
                pb.set_style(
                    ProgressStyle::default_bar()
                        .template("{prefix} [{bar}] {bytes}/{total_bytes} eta: {eta}")
                        .unwrap()
                        .progress_chars("=> "),
                );
                Some(pb)
            } else {
                None
            };

            let last_modified = response
                .headers()
                .get("etag")
                .map(|etag| etag.to_str().unwrap_or_default().to_string())
                .unwrap_or_else(|| "unknown_etag".to_string());

            if let Some(parent) = target_file_path.parent() {
                std::fs::create_dir_all(parent).with_context(|| {
                    format!(
                        "Failed to create parent directory for binary: {}",
                        parent.display()
                    )
                })?;
            }
            let mut dest_file = File::create(target_file_path).with_context(|| {
                format!(
                    "Failed to create target file for binary: {}",
                    target_file_path.display()
                )
            })?;

            let mut source = if let Some(ref pb) = pb {
                Box::new(pb.wrap_read(response)) as Box<dyn std::io::Read>
            } else {
                Box::new(response) as Box<dyn std::io::Read>
            };
            std::io::copy(&mut source, &mut dest_file)?;

            // On Unix, make the binary executable
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let perms = std::fs::Permissions::from_mode(0o755); // rwxr-xr-x
                std::fs::set_permissions(target_file_path, perms)?;
            }

            Ok(last_modified)
        },
        url,
    )
}

#[cfg(windows)]
struct DataReaderWrap(windows::Storage::Streams::DataReader);

#[cfg(windows)]
impl std::io::Read for DataReaderWrap {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let mut bytes =
            self.0
                .LoadAsync(buf.len() as u32)
                .map_err(|e| std::io::Error::from_raw_os_error(e.code().0))?
                .get()
                .map_err(|e| std::io::Error::from_raw_os_error(e.code().0))? as usize;
        bytes = bytes.min(buf.len());
        self.0
            .ReadBytes(&mut buf[0..bytes])
            .map_err(|e| std::io::Error::from_raw_os_error(e.code().0))
            .map(|_| bytes)
    }
}

#[cfg(windows)]
pub fn download_extract_sans_parent(
    url: &str,
    target_path: &Path,
    levels_to_skip: usize,
) -> Result<String> {
    use windows::core::HSTRING;

    let http_client =
        windows::Web::Http::HttpClient::new().with_context(|| "Failed to create HttpClient.")?;

    let request_uri = windows::Foundation::Uri::CreateUri(&windows::core::HSTRING::from(url))
        .with_context(|| "Failed to convert url string to Uri.")?;

    let http_response = http_client
        .GetAsync(&request_uri)
        .with_context(|| "Failed to initiate download.")?
        .get()
        .with_context(|| "Failed to complete async download operation.")?;

    http_response
        .EnsureSuccessStatusCode()
        .with_context(|| "HTTP download reported error status code.")?;

    let last_modified = http_response
        .Headers()
        .unwrap() // This unwrap could panic
        .Lookup(&HSTRING::from("etag"))
        .unwrap() // This unwrap could panic
        .to_string();

    let http_response_content = http_response
        .Content()
        .with_context(|| "Failed to obtain content from http response.")?;

    let response_stream = http_response_content
        .ReadAsInputStreamAsync()
        .with_context(|| "Failed to initiate get input stream from response")?
        .get()
        .with_context(|| "Failed to obtain input stream from http response")?;

    let reader = windows::Storage::Streams::DataReader::CreateDataReader(&response_stream)
        .with_context(|| "Failed to create DataReader.")?;

    reader
        .SetInputStreamOptions(windows::Storage::Streams::InputStreamOptions::ReadAhead)
        .with_context(|| "Failed to set input stream options.")?;

    let mut content_length: u64 = 0;

    // Check CI context to determine if progress bar should be shown
    let ci_context = CiContext::detect();
    let pb = if ci_context.should_show_progress() {
        let pb = if http_response_content.TryComputeLength(&mut content_length)? {
            ProgressBar::new(content_length)
        } else {
            ProgressBar::new_spinner()
        };
        pb.set_prefix(style("  Downloading:").cyan().bold().to_string());
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{prefix} [{bar}] {bytes}/{total_bytes} eta: {eta}")
                .unwrap()
                .progress_chars("=> "),
        );
        Some(pb)
    } else {
        None
    };

    let response_reader = if let Some(ref pb) = pb {
        pb.wrap_read(DataReaderWrap(reader))
    } else {
        DataReaderWrap(reader)
    };

    unpack_sans_parent(response_reader, target_path, levels_to_skip)
        .with_context(|| format!("Failed to extract downloaded file from url `{}`.", url))?;

    Ok(last_modified)
}

#[cfg(windows)]
pub fn download_extract_zip(
    url: &str,
    target_path: &Path,
    levels_to_skip: usize,
) -> Result<String> {
    use std::io::Seek;
    use windows::core::HSTRING; // Required for temp_file.seek

    let http_client = windows::Web::Http::HttpClient::new()
        .with_context(|| "Failed to create HttpClient for ZIP.")?;
    let request_uri = windows::Foundation::Uri::CreateUri(&HSTRING::from(url))
        .with_context(|| "Failed to convert ZIP url string to Uri.")?;
    let http_response = http_client
        .GetAsync(&request_uri)
        .with_context(|| "Failed to initiate ZIP download.")?
        .get()
        .with_context(|| "Failed to complete async ZIP download operation.")?;

    http_response
        .EnsureSuccessStatusCode()
        .with_context(|| "HTTP ZIP download reported error status code.")?;

    let last_modified = http_response
        .Headers()
        .context("Failed to get headers from ZIP response")?
        .Lookup(&HSTRING::from("etag"))
        .map(|h_str| h_str.to_string())
        .unwrap_or_else(|| "unknown_etag".to_string());

    let http_response_content = http_response
        .Content()
        .with_context(|| "Failed to obtain content from HTTP ZIP response.")?;

    let mut content_length: u64 = 0;

    // Check CI context to determine if progress bar should be shown
    let ci_context = CiContext::detect();
    let pb = if ci_context.should_show_progress() {
        let pb = if http_response_content.TryComputeLength(&mut content_length)? {
            ProgressBar::new(content_length)
        } else {
            ProgressBar::new_spinner()
        };
        pb.set_prefix(style("  Downloading ZIP:").cyan().bold().to_string());
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{prefix} [{bar}] {bytes}/{total_bytes} eta: {eta}")
                .unwrap()
                .progress_chars("=> "),
        );
        Some(pb)
    } else {
        None
    };

    // Download to a temporary file first as ZipArchive needs Read + Seek
    let mut temp_file = tempfile::Builder::new().suffix(".zip").tempfile()?;

    let response_stream = http_response_content
        .ReadAsInputStreamAsync()
        .with_context(|| "Failed to initiate get input stream from ZIP response")?
        .get()
        .with_context(|| "Failed to obtain input stream from HTTP ZIP response")?;
    let reader = windows::Storage::Streams::DataReader::CreateDataReader(&response_stream)
        .with_context(|| "Failed to create DataReader for ZIP.")?;
    reader
        .SetInputStreamOptions(windows::Storage::Streams::InputStreamOptions::ReadAhead)
        .with_context(|| "Failed to set input stream options for ZIP.")?;

    let mut response_reader = if let Some(ref pb) = pb {
        pb.wrap_read(DataReaderWrap(reader))
    } else {
        DataReaderWrap(reader)
    };
    std::io::copy(&mut response_reader, &mut temp_file)?; // temp_file is NamedTempFile
    temp_file.seek(SeekFrom::Start(0))?; // temp_file (as File) seek

    unpack_zip(temp_file, target_path, levels_to_skip) // unpack_zip expects R: Read + Seek
        .with_context(|| format!("Failed to extract downloaded ZIP file from url `{}`.", url))?;

    Ok(last_modified)
}

#[cfg(windows)]
pub fn download_binary(url: &str, target_file_path: &Path) -> Result<String> {
    use windows::core::HSTRING;

    let http_client = windows::Web::Http::HttpClient::new()
        .with_context(|| "Failed to create HttpClient for binary.")?;
    let request_uri = windows::Foundation::Uri::CreateUri(&HSTRING::from(url))
        .with_context(|| "Failed to convert binary url string to Uri.")?;
    let http_response = http_client
        .GetAsync(&request_uri)
        .with_context(|| "Failed to initiate binary download.")?
        .get()
        .with_context(|| "Failed to complete async binary download operation.")?;

    http_response
        .EnsureSuccessStatusCode()
        .with_context(|| "HTTP binary download reported error status code.")?;

    let last_modified = http_response
        .Headers()
        .context("Failed to get headers from binary response")?
        .Lookup(&HSTRING::from("etag"))
        .map(|h_str| h_str.to_string())
        .unwrap_or_else(|| "unknown_etag".to_string());

    let http_response_content = http_response
        .Content()
        .with_context(|| "Failed to obtain content from HTTP binary response.")?;

    let mut content_length: u64 = 0;

    // Check CI context to determine if progress bar should be shown
    let ci_context = CiContext::detect();
    let pb = if ci_context.should_show_progress() {
        let pb = if http_response_content.TryComputeLength(&mut content_length)? {
            ProgressBar::new(content_length)
        } else {
            ProgressBar::new_spinner()
        };
        pb.set_prefix(style("  Downloading Binary:").cyan().bold().to_string());
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{prefix} [{bar}] {bytes}/{total_bytes} eta: {eta}")
                .unwrap()
                .progress_chars("=> "),
        );
        Some(pb)
    } else {
        None
    };

    if let Some(parent) = target_file_path.parent() {
        std::fs::create_dir_all(parent).with_context(|| {
            format!(
                "Failed to create parent directory for binary: {}",
                parent.display()
            )
        })?;
    }
    let mut dest_file = File::create(target_file_path).with_context(|| {
        format!(
            "Failed to create target file for binary: {}",
            target_file_path.display()
        )
    })?;

    let response_stream = http_response_content
        .ReadAsInputStreamAsync()
        .with_context(|| "Failed to initiate get input stream from binary response")?
        .get()
        .with_context(|| "Failed to obtain input stream from HTTP binary response")?;
    let reader = windows::Storage::Streams::DataReader::CreateDataReader(&response_stream)
        .with_context(|| "Failed to create DataReader for binary.")?;
    reader
        .SetInputStreamOptions(windows::Storage::Streams::InputStreamOptions::ReadAhead)
        .with_context(|| "Failed to set input stream options for binary.")?;

    let mut source = if let Some(ref pb) = pb {
        pb.wrap_read(DataReaderWrap(reader))
    } else {
        DataReaderWrap(reader)
    };
    std::io::copy(&mut source, &mut dest_file)?;

    // No specific executable bit to set on Windows for typical .exe files
    Ok(last_modified)
}
