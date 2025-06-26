use crate::global_paths::GupGlobalPaths;
use crate::operations_download::{
    download_binary, download_extract_sans_parent, download_extract_zip,
};
use crate::operations_metadata::{get_current_platform_id, MergedInstallConfig};
use crate::project_metadata_m::ProjectMetadata;
use crate::state_config::{InstalledVersionConcreteInfo, ProjectLocalState};
use anyhow::{anyhow, bail, Context, Result};
use chrono::Utc;
use console::style;
// use std::path::PathBuf; // PathBuf is used implicitly via .join()
// use url::Url; // Url is used as a type in ProjectMetadata, not directly here

pub fn install_version(
    project_unique_name: &str,             // New parameter
    version_to_install: &str,              // Renamed from fullversion
    project_metadata: &ProjectMetadata,    // New parameter, replaces version_db
    project_state: &mut ProjectLocalState, // New parameter, replaces config_data
    paths: &GupGlobalPaths,                // Changed from GlobalPaths
) -> Result<()> {
    // Return immediately if the version is already installed.
    if project_state
        .installed_versions
        .contains_key(version_to_install)
    {
        eprintln!(
            "{} {} version {} is already installed.",
            project_metadata.project_name,
            style("Info:").bold(),
            version_to_install
        );
        return Ok(());
    }

    eprintln!(
        "{} {} version {} for project {}.",
        style("Starting installation of").cyan().bold(),
        project_metadata.project_name,
        version_to_install,
        project_unique_name
    );

    // 1. Determine current platform_id
    let platform_id = get_current_platform_id(project_metadata).with_context(|| {
        format!(
            "Failed to determine a compatible platform for project '{}' on this system.",
            project_metadata.project_name
        )
    })?;
    eprintln!("  Detected platform_id: {}", style(&platform_id).green());

    // 2. Get VersionDetail for version_to_install
    let version_detail = project_metadata
        .available_versions
        .get(version_to_install)
        .ok_or_else(|| {
            anyhow!(
                "Version '{}' not found in available_versions for project '{}'.",
                version_to_install,
                project_metadata.project_name
            )
        })?;

    // 3. Get ArtifactDetail for the platform_id from VersionDetail.artifacts
    let artifact_detail = version_detail.artifacts.get(&platform_id).ok_or_else(|| {
        anyhow!(
            "No artifact found for platform_id '{}' in version '{}' of project '{}'.",
            platform_id,
            version_to_install,
            project_metadata.project_name
        )
    })?;

    // 4. Construct download URL
    let download_url = project_metadata
        .base_download_url
        .join(&artifact_detail.url_path_suffix)
        .with_context(|| {
            format!(
                "Failed to construct download URL from base '{}' and suffix '{}'",
                project_metadata.base_download_url, artifact_detail.url_path_suffix
            )
        })?;
    eprintln!("  Download URL: {}", style(download_url.as_str()).green());

    // 5. Determine installation parameters
    let merged_config = MergedInstallConfig::new(
        &project_metadata.default_install_config,
        &version_detail.install_config_override,
        // &artifact_detail.install_config_override, // Removed, ArtifactDetail does not have this field
    );
    eprintln!("  Merged Install Config: {:?}", merged_config);

    // 6. Create target directory:
    let path_segment = format!("{}-{}", version_to_install, platform_id);
    let target_install_dir = paths
        .project_versions_dir(project_unique_name)
        .join(&path_segment);

    eprintln!(
        "  Target installation directory: {}",
        style(target_install_dir.display()).green()
    );

    if target_install_dir.exists() {
        std::fs::remove_dir_all(&target_install_dir).with_context(|| {
            format!(
                "Failed to remove existing target directory before installation: {}",
                target_install_dir.display()
            )
        })?;
    }
    std::fs::create_dir_all(&target_install_dir).with_context(|| {
        format!(
            "Failed to create target installation directory: {}",
            target_install_dir.display()
        )
    })?;

    // 7. Download and extract based on merged_config
    let etag_or_hash = match merged_config.archive_format.as_str() {
        "tar.gz" => {
            eprintln!(
                "  Downloading and extracting tar.gz archive from {} with strip_components = {}...",
                download_url, merged_config.strip_components
            );
            download_extract_sans_parent(
                download_url.as_str(),
                &target_install_dir,
                merged_config.strip_components,
            )
            .with_context(|| {
                format!(
                    "Failed to download and extract tar.gz from {} to {}",
                    download_url,
                    target_install_dir.display()
                )
            })?
        }
        "zip" => {
            eprintln!(
                "  Downloading and extracting zip archive from {} with strip_components = {}...",
                download_url, merged_config.strip_components
            );
            download_extract_zip(
                download_url.as_str(),
                &target_install_dir,
                merged_config.strip_components, // strip_components is handled by unpack_zip
            )
            .with_context(|| {
                format!(
                    "Failed to download and extract zip from {} to {}",
                    download_url,
                    target_install_dir.display()
                )
            })?
        }
        "binary" => {
            // For binary, the url_path_suffix is assumed to be the filename or relative path
            // of the binary itself. It will be placed inside target_install_dir.
            let binary_target_path = target_install_dir.join(&artifact_detail.url_path_suffix);
            eprintln!(
                "  Downloading binary from {} to {}...",
                download_url,
                binary_target_path.display()
            );
            download_binary(download_url.as_str(), &binary_target_path).with_context(|| {
                format!(
                    "Failed to download binary from {} to {}",
                    download_url,
                    binary_target_path.display()
                )
            })?
        }
        unsupported_format => {
            bail!(
                "Unsupported archive_format '{}' for project '{}', version '{}'.",
                unsupported_format,
                project_metadata.project_name,
                version_to_install
            );
        }
    };
    eprintln!(
        "  Successfully downloaded and extracted. ETag/Hash: {}",
        style(&etag_or_hash).green()
    );

    // 8. Update project_state.installed_versions
    project_state.installed_versions.insert(
        version_to_install.to_string(),
        InstalledVersionConcreteInfo {
            version_string: version_to_install.to_string(),
            path_segment,
            installed_at: Utc::now(),
        },
    );
    eprintln!(
        "{} {} version {} marked as installed in state.",
        style("Successfully").green().bold(),
        project_metadata.project_name,
        version_to_install
    );

    // 9. Run post-install hook, if any
    if let Some(hook_command_template) = merged_config.post_install_hook {
        if !hook_command_template.trim().is_empty() {
            eprintln!(
                "  Running post-install hook: '{}'",
                style(&hook_command_template).yellow()
            );

            let hook_command_actual = hook_command_template.replace(
                "{GUP_PROJECT_VERSION_DIR}",
                target_install_dir.to_string_lossy().as_ref(),
            );

            let (shell, arg_flag) = if cfg!(windows) {
                ("cmd", "/C")
            } else {
                ("sh", "-c")
            };

            let mut command_parts = hook_command_actual.split_whitespace();
            let program = command_parts.next().unwrap_or(&hook_command_actual);
            let args: Vec<&str> = command_parts.collect();

            let mut process_builder = if program == shell || hook_command_actual.contains(&arg_flag)
            {
                let mut cmd = std::process::Command::new(shell);
                cmd.arg(arg_flag).arg(&hook_command_actual);
                cmd
            } else {
                let mut cmd = std::process::Command::new(program);
                cmd.args(args);
                cmd
            };

            let status = process_builder
                .current_dir(&target_install_dir)
                .status()
                .with_context(|| {
                    format!(
                        "Failed to execute post-install hook: `{}` in directory {}",
                        hook_command_actual,
                        target_install_dir.display()
                    )
                })?;

            if status.success() {
                eprintln!("  Post-install hook completed successfully.");
            } else {
                bail!(
                    "Post-install hook failed with status: {}. Command: `{}` in directory {}",
                    status,
                    hook_command_actual,
                    target_install_dir.display()
                );
            }
        }
    }
    Ok(())
}
