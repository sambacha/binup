#![cfg(feature = "selfupdate")]

use crate::constants::{APP_NAME, SELFUPDATE_CRON_MARKER};
use anyhow::{anyhow, bail, Context, Result};
use itertools::Itertools;
use std::io::Write;
use std::process::Stdio;

/// Download and install a gup update
pub fn download_and_install_gup_update(
    new_version: &str,
    target_platform: &str,
    download_base_url: &str,
) -> Result<()> {
    use crate::operations_download::download_extract_sans_parent;

    // Get current executable path
    let current_exe =
        std::env::current_exe().with_context(|| "Failed to get current executable path")?;

    // Construct download URL
    let download_url = format!(
        "{}/v{}/gup-{}-{}.tar.gz",
        download_base_url.trim_end_matches('/'),
        new_version.trim_start_matches('v'),
        new_version.trim_start_matches('v'),
        target_platform
    );

    // Create temporary directory for download
    let temp_dir =
        tempfile::tempdir().with_context(|| "Failed to create temporary directory for update")?;

    eprintln!("Downloading gup {} for {}...", new_version, target_platform);

    // Download and extract
    download_extract_sans_parent(&download_url, temp_dir.path(), 1).with_context(|| {
        format!(
            "Failed to download gup version {} from {}",
            new_version, download_url
        )
    })?;

    // Find the new gup binary in extracted files
    let new_gup_binary = temp_dir.path().join("gup");
    if !new_gup_binary.exists() {
        // Try with .exe extension on Windows
        let new_gup_binary_exe = temp_dir.path().join("gup.exe");
        if new_gup_binary_exe.exists() {
            perform_binary_replacement(&current_exe, &new_gup_binary_exe, new_version)?;
        } else {
            bail!("Downloaded archive does not contain expected gup binary");
        }
    } else {
        perform_binary_replacement(&current_exe, &new_gup_binary, new_version)?;
    }

    Ok(())
}

fn perform_binary_replacement(
    current_exe: &std::path::Path,
    new_binary: &std::path::Path,
    new_version: &str,
) -> Result<()> {
    // Make it executable on Unix systems
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o755);
        std::fs::set_permissions(new_binary, perms)
            .with_context(|| "Failed to make new gup binary executable")?;
    }

    // Create backup of current binary
    let backup_path = current_exe.with_extension("backup");
    std::fs::copy(current_exe, &backup_path)
        .with_context(|| "Failed to create backup of current gup binary")?;

    // On Windows, we need to rename the current executable first
    #[cfg(windows)]
    {
        let temp_old_path = current_exe.with_extension("old");
        std::fs::rename(current_exe, &temp_old_path)
            .with_context(|| "Failed to rename current executable")?;

        // Copy new binary to current location
        match std::fs::copy(new_binary, current_exe) {
            Ok(_) => {
                // Clean up old binary
                let _ = std::fs::remove_file(&temp_old_path);
            }
            Err(e) => {
                // Restore on error
                let _ = std::fs::rename(&temp_old_path, current_exe);
                return Err(e).with_context(|| "Failed to copy new binary");
            }
        }
    }

    // On Unix, we can directly replace
    #[cfg(not(windows))]
    {
        std::fs::copy(new_binary, current_exe)
            .with_context(|| "Failed to replace current gup binary with new version")?;
    }

    // Verify the new binary works
    let output = std::process::Command::new(current_exe)
        .arg("--version")
        .output()
        .with_context(|| "Failed to verify new gup binary")?;

    if !output.status.success() {
        // Restore backup if verification failed
        std::fs::copy(&backup_path, current_exe)
            .with_context(|| "Failed to restore backup after verification failure")?;
        bail!("New gup binary verification failed, restored previous version");
    }

    // Clean up backup file
    let _ = std::fs::remove_file(&backup_path);

    eprintln!("Successfully updated gup to version {}", new_version);
    Ok(())
}

/// Check for gup updates from GitHub releases
pub fn check_for_gup_updates(
    current_version: &str,
    channel: &str,
    github_org: &str,
    github_repo: &str,
) -> Result<Option<String>> {
    // Construct API URL based on channel
    let api_url = match channel {
        "release" => format!(
            "https://api.github.com/repos/{}/{}/releases/latest",
            github_org, github_repo
        ),
        "releasepreview" => format!(
            "https://api.github.com/repos/{}/{}/releases?per_page=1&prerelease=true",
            github_org, github_repo
        ),
        "dev" => format!(
            "https://api.github.com/repos/{}/{}/releases?per_page=1",
            github_org, github_repo
        ),
        _ => bail!("Unknown update channel: {}", channel),
    };

    // Set up request with user agent (GitHub API requires this)
    let client = reqwest::blocking::Client::builder()
        .user_agent("gup-updater")
        .build()
        .with_context(|| "Failed to create HTTP client")?;

    let response = client
        .get(&api_url)
        .send()
        .with_context(|| format!("Failed to query GitHub API for {} channel", channel))?;

    if response.status() == 404 {
        // Repository not found or no releases - not an error, just no updates
        return Ok(None);
    }

    if !response.status().is_success() {
        bail!("GitHub API returned status {}", response.status());
    }

    let release_info: serde_json::Value = response
        .json()
        .with_context(|| "Failed to parse GitHub API response")?;

    let latest_version = if channel == "releasepreview" || channel == "dev" {
        // For arrays, get the first element
        release_info
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|release| release["tag_name"].as_str())
    } else {
        // For single release object
        release_info["tag_name"].as_str()
    }
    .ok_or_else(|| anyhow!("Could not find version information in GitHub API response"))?;

    // Remove 'v' prefix if present for comparison
    let latest_version = latest_version.strip_prefix('v').unwrap_or(latest_version);
    let current_version = current_version.strip_prefix('v').unwrap_or(current_version);

    // Compare versions
    match (
        semver::Version::parse(current_version),
        semver::Version::parse(latest_version),
    ) {
        (Ok(current), Ok(latest)) => {
            if latest > current {
                Ok(Some(latest.to_string()))
            } else {
                Ok(None)
            }
        }
        _ => {
            // Fallback to string comparison if semver parsing fails
            if latest_version != current_version {
                Ok(Some(latest_version.to_string()))
            } else {
                Ok(None)
            }
        }
    }
}

pub fn install_background_selfupdate(interval: i64) -> Result<()> {
    let own_exe_path = std::env::current_exe()
        .with_context(|| "Could not determine the path of the running exe.")?;

    let my_own_path = own_exe_path.to_str().unwrap();

    match std::env::var("WSL_DISTRO_NAME") {
        // This is the WSL case, where we schedule a Windows task to do the update
        Ok(val) => {
            std::process::Command::new("schtasks.exe")
                .args([
                    "/create",
                    "/sc",
                    "minute",
                    "/mo",
                    &interval.to_string(),
                    "/tn",
                    &format!("{} self update for WSL {} distribution", APP_NAME, val),
                    "/f",
                    "/it",
                    "/tr",
                    &format!("wsl --distribution {} {} self update", val, my_own_path),
                ])
                .output()
                .with_context(|| "Failed to create new Windows task for gup.")?;
        }
        Err(_e) => {
            let output = std::process::Command::new("crontab")
                .args(["-l"])
                .output()
                .with_context(|| "Failed to retrieve crontab configuration.")?;

            let new_crontab_content = String::from_utf8(output.stdout)?
                .lines()
                .filter(|x| !x.contains(SELFUPDATE_CRON_MARKER))
                .chain([
                    &format!(
                        "*/{} * * * * {} self update {}",
                        interval, my_own_path, SELFUPDATE_CRON_MARKER
                    ),
                    "",
                ])
                .join("\n");

            let mut child = std::process::Command::new("crontab")
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .spawn()?;

            let mut child_stdin = child.stdin.take().unwrap();

            child_stdin.write_all(new_crontab_content.as_bytes())?;

            // Close stdin to finish and avoid indefinite blocking
            drop(child_stdin);

            child.wait_with_output()?;
        }
    };

    Ok(())
}

pub fn uninstall_background_selfupdate() -> Result<()> {
    match std::env::var("WSL_DISTRO_NAME") {
        // This is the WSL case, where we schedule a Windows task to do the update
        Ok(val) => {
            std::process::Command::new("schtasks.exe")
                .args([
                    "/delete",
                    "/tn",
                    &format!("{} self update for WSL {} distribution", APP_NAME, val),
                    "/f",
                ])
                .output()
                .with_context(|| "Failed to remove Windows task for gup.")?;
        }
        Err(_e) => {
            let output = std::process::Command::new("crontab")
                .args(["-l"])
                .output()
                .with_context(|| "Failed to remove cron task.")?;

            let new_crontab_content = String::from_utf8(output.stdout)?
                .lines()
                .filter(|x| !x.contains(SELFUPDATE_CRON_MARKER))
                .chain([""])
                .join("\n");

            let mut child = std::process::Command::new("crontab")
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .spawn()?;

            let mut child_stdin = child.stdin.take().unwrap();

            child_stdin.write_all(new_crontab_content.as_bytes())?;

            // Close stdin to finish and avoid indefinite blocking
            drop(child_stdin);

            child.wait_with_output()?;
        }
    };

    Ok(())
}
