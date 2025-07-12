use crate::error_handling::GupError;
use crate::global_paths::GupGlobalPaths;
use crate::project_metadata_m::{InstallConfig, ProjectMetadata};
use crate::utils::get_target_triple_id;
use anyhow::{anyhow, bail, Context, Result};
use console::style;
use semver::Version;
use url::Url;

// New function to sync project metadata
pub fn sync_project_metadata(
    project_unique_name: &str,
    source_url: &Url,
    paths: &GupGlobalPaths,
) -> Result<ProjectMetadata> {
    eprintln!(
        "{} metadata for project {} from {}.",
        style("Syncing").cyan().bold(),
        project_unique_name,
        source_url
    );

    let response = reqwest::blocking::get(source_url.clone())
        .with_context(|| format!("Failed to download metadata from url `{}`.", source_url))?;

    if !response.status().is_success() {
        bail!(
            "Failed to download metadata from {}: HTTP status {}",
            source_url,
            response.status()
        );
    }

    let metadata_content = response
        .text()
        .with_context(|| format!("Failed to read metadata content from {}.", source_url))?;

    let metadata: ProjectMetadata = serde_json::from_str(&metadata_content)
        .with_context(|| format!("Failed to parse project metadata from {}.", source_url))?;

    // Validate that the project name in the metadata matches what we expect (optional but good practice)
    if metadata.project_name != project_unique_name {
        // This check might be too strict if project_unique_name can differ from metadata.project_name
        // For example, if project_unique_name is a user-defined alias.
        // Consider if this validation is appropriate for your use case.
        // For now, we'll log a warning instead of bailing.
        log::warn!(
            "Project name in metadata ('{}') does not match unique name ('{}'). Using metadata name.",
            metadata.project_name,
            project_unique_name
        );
    }

    let cache_file_path = paths.project_metadata_cache_file(project_unique_name);
    if let Some(parent_dir) = cache_file_path.parent() {
        std::fs::create_dir_all(parent_dir).with_context(|| {
            format!(
                "Failed to create directory for metadata cache at {}.",
                parent_dir.display()
            )
        })?;
    }

    std::fs::write(&cache_file_path, metadata_content.as_bytes()).with_context(|| {
        format!(
            "Failed to write project metadata cache to {}.",
            cache_file_path.display()
        )
    })?;

    eprintln!(
        "{} metadata for {} cached at {}.",
        style("Successfully synced").green().bold(),
        project_unique_name, // Or metadata.project_name
        cache_file_path.display()
    );

    Ok(metadata)
}

// Helper struct to hold the merged installation configuration
#[derive(Debug, Clone)]
pub(crate) struct MergedInstallConfig {
    // Made pub(crate) as it's used by install_version in operations.rs (soon operations_install.rs)
    pub(crate) archive_format: String, // e.g., "tar.gz", "zip", "binary"
    pub(crate) strip_components: usize,
    pub(crate) bin_subdir: Option<String>, // Relative path from extracted root to directory containing executables
    pub(crate) post_install_hook: Option<String>, // Command to run after successful installation
                                           // Add other relevant fields like specific_executable_name etc.
}

impl MergedInstallConfig {
    pub(crate) fn new(
        default_config: &InstallConfig, // Changed from &Option<InstallConfig>
        version_override_config: &Option<InstallConfig>,
        // artifact_override_config: &Option<InstallConfig>, // Removed this parameter
    ) -> Self {
        // Start with values from the default_config (which is no longer an Option)
        let mut final_config = MergedInstallConfig {
            archive_format: default_config
                .archive_format
                .clone()
                .unwrap_or_else(|| "tar.gz".to_string()),
            strip_components: default_config.strip_components.unwrap_or(0),
            bin_subdir: default_config.bin_subdir.clone(),
            post_install_hook: default_config.post_install_hook.clone(),
        };

        // Layer 2: Version-level install_config_override
        if let Some(vc) = version_override_config {
            if let Some(val) = &vc.archive_format {
                final_config.archive_format = val.clone();
            }
            if let Some(val) = vc.strip_components {
                final_config.strip_components = val;
            }
            if vc.bin_subdir.is_some() {
                final_config.bin_subdir = vc.bin_subdir.clone();
            }
            if vc.post_install_hook.is_some() {
                final_config.post_install_hook = vc.post_install_hook.clone();
            }
        }

        // Layer 3: Artifact-level install_config_override (Removed as parameter is removed)
        // if let Some(ac) = artifact_override_config {
        //     if let Some(val) = &ac.archive_format {
        //         final_config.archive_format = val.clone();
        //     }
        //     if let Some(val) = ac.strip_components {
        //         final_config.strip_components = val;
        //     }
        //     if ac.bin_subdir.is_some() {
        //         final_config.bin_subdir = ac.bin_subdir.clone();
        //     }
        //     if ac.post_install_hook.is_some() {
        //         final_config.post_install_hook = ac.post_install_hook.clone();
        //     }
        // }

        final_config
    }
}

// New function to resolve a channel name to a concrete version string
pub fn resolve_channel_to_version(
    channel_name: &str,
    metadata: &ProjectMetadata,
) -> Result<String> {
    eprintln!(
        "{} channel '{}' for project {}.",
        style("Resolving").cyan().bold(),
        channel_name,
        metadata.project_name
    );

    // 1. Check if channel_name is a defined channel in metadata
    if let Some(channel_detail) = metadata.channels.get(channel_name) {
        match channel_detail.resolution_strategy.as_str() {
            "latest_semver" => {
                let mut candidate_versions: Vec<Version> = Vec::new();
                for (version_str, _version_detail) in &metadata.available_versions {
                    if let Some(prefix) = &channel_detail.version_prefix {
                        if !version_str.starts_with(prefix) {
                            continue;
                        }
                    }
                    if let Ok(sem_ver) = Version::parse(version_str) {
                        candidate_versions.push(sem_ver);
                    }
                }
                candidate_versions.sort_unstable(); // Sorts ascending
                if let Some(latest_version) = candidate_versions.last() {
                    eprintln!(
                        "Resolved channel '{}' to version '{}' using latest_semver strategy.",
                        channel_name,
                        latest_version.to_string()
                    );
                    return Ok(latest_version.to_string());
                } else {
                    let available_versions: Vec<String> =
                        metadata.available_versions.keys().cloned().collect();
                    return Err(GupError::version_not_found(
                        &metadata.project_name,
                        channel_name,
                        &available_versions,
                    ));
                }
            }
            "exact_match" => {
                // For "exact_match", the channel_detail.version_prefix IS the version.
                let version_to_match = channel_detail.version_prefix.as_ref().ok_or_else(|| {
                    anyhow!(
                        "Channel '{}' with 'exact_match' strategy is missing 'version_prefix'.",
                        channel_name
                    )
                })?;
                if metadata.available_versions.contains_key(version_to_match) {
                    eprintln!(
                        "Resolved channel '{}' to version '{}' using exact_match strategy.",
                        channel_name, version_to_match
                    );
                    return Ok(version_to_match.clone());
                } else {
                    bail!(
                        "Channel '{}' (strategy: exact_match) requires version '{}', which is not available for project {}.",
                        channel_name,
                        version_to_match,
                        metadata.project_name
                    );
                }
            }
            unknown_strategy => {
                bail!(
                    "Unknown resolution strategy '{}' for channel '{}' in project {}.",
                    unknown_strategy,
                    channel_name,
                    metadata.project_name
                );
            }
        }
    }

    // 2. If not a defined channel, check if channel_name is a direct version string
    if metadata.available_versions.contains_key(channel_name) {
        eprintln!(
            "Interpreting '{}' as a direct version string for project {}.",
            channel_name, metadata.project_name
        );
        return Ok(channel_name.to_string());
    }

    let available_versions: Vec<String> = metadata.available_versions.keys().cloned().collect();
    Err(GupError::version_not_found(
        &metadata.project_name,
        channel_name,
        &available_versions,
    ))
}

// Helper function to determine the current platform_id based on ProjectMetadata
pub(crate) fn get_current_platform_id(metadata: &ProjectMetadata) -> Result<String> {
    // Made pub(crate) as it's used by install_version in operations.rs (soon operations_install.rs)
    let current_triple = get_target_triple_id()
        .with_context(|| "Failed to determine current system target triple.")?;

    if metadata.platforms.contains_key(&current_triple) {
        return Ok(current_triple);
    }

    bail!(
        "No matching platform_id found in project metadata for current system triple '{}'. Available platforms: {:?}",
        current_triple,
        metadata.platforms.keys().collect::<Vec<_>>()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project_metadata_m::*;
    use std::collections::HashMap;

    fn create_test_metadata() -> ProjectMetadata {
        let mut platforms = HashMap::new();
        platforms.insert(
            "linux-x64".to_string(),
            PlatformDetail {
                os: "linux".to_string(),
                arch: "x86_64".to_string(),
                target_triple_pattern: "x86_64.*linux.*".to_string(),
            },
        );

        let mut channels = HashMap::new();
        channels.insert(
            "stable".to_string(),
            ChannelDetail {
                version_prefix: None,
                resolution_strategy: "latest_semver".to_string(),
            },
        );
        channels.insert(
            "v1.0".to_string(),
            ChannelDetail {
                version_prefix: Some("1.0.".to_string()),
                resolution_strategy: "latest_semver".to_string(),
            },
        );

        let mut versions = HashMap::new();
        versions.insert(
            "1.0.0".to_string(),
            VersionDetail {
                release_date: Some("2024-01-01T00:00:00Z".to_string()),
                artifacts: HashMap::new(),
                install_config_override: None,
            },
        );
        versions.insert(
            "1.0.1".to_string(),
            VersionDetail {
                release_date: Some("2024-01-15T00:00:00Z".to_string()),
                artifacts: HashMap::new(),
                install_config_override: None,
            },
        );
        versions.insert(
            "1.1.0".to_string(),
            VersionDetail {
                release_date: Some("2024-02-01T00:00:00Z".to_string()),
                artifacts: HashMap::new(),
                install_config_override: None,
            },
        );

        ProjectMetadata {
            project_name: "test-project".to_string(),
            display_name: Some("Test Project".to_string()),
            description: Some("A test project".to_string()),
            homepage_url: Some("https://example.com".to_string()),
            metadata_format_version: "1.0.0".to_string(),
            base_download_url: url::Url::parse("https://example.com/releases/").unwrap(),
            platforms,
            default_install_config: InstallConfig::default(),
            executables: vec![ExecutableDetail {
                name: "test".to_string(),
                path_in_bin_subdir: "test".to_string(),
            }],
            default_executable_name: Some("test".to_string()),
            available_versions: versions,
            channels,
        }
    }

    #[test]
    fn test_resolve_channel_latest_semver() {
        let metadata = create_test_metadata();

        // Test latest_semver resolution
        let result = resolve_channel_to_version("stable", &metadata);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "1.1.0"); // Should pick latest version
    }

    #[test]
    fn test_resolve_channel_with_prefix() {
        let metadata = create_test_metadata();

        // Test prefix-based resolution
        let result = resolve_channel_to_version("v1.0", &metadata);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "1.0.1"); // Should pick latest 1.0.x
    }

    #[test]
    fn test_resolve_direct_version() {
        let metadata = create_test_metadata();

        // Test direct version resolution
        let result = resolve_channel_to_version("1.0.0", &metadata);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "1.0.0");
    }

    #[test]
    fn test_resolve_unknown_channel() {
        let metadata = create_test_metadata();

        // Test unknown channel
        let result = resolve_channel_to_version("nonexistent", &metadata);
        assert!(result.is_err());
    }
}
