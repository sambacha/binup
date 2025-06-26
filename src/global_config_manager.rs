use anyhow::{Context, Result};
use std::fs;

use crate::global_paths::GupGlobalPaths;
use crate::state_config::GupGlobalConfig;

pub fn load_global_config(paths: &GupGlobalPaths) -> Result<GupGlobalConfig> {
    let config_path = paths.global_config_file();

    if !config_path.exists() {
        // If the config file doesn't exist, return a default configuration
        return Ok(GupGlobalConfig::default());
    }

    let content = fs::read_to_string(&config_path).with_context(|| {
        format!(
            "Could not read global config file at '{}'.",
            config_path.display()
        )
    })?;

    let config: GupGlobalConfig = serde_json::from_str(&content).with_context(|| {
        format!(
            "Could not parse global config file at '{}'.",
            config_path.display()
        )
    })?;

    Ok(config)
}

pub fn save_global_config(config: &GupGlobalConfig, paths: &GupGlobalPaths) -> Result<()> {
    let config_path = paths.global_config_file();
    let parent_dir = config_path.parent().ok_or_else(|| {
        anyhow::anyhow!("Could not determine parent directory for global config file.")
    })?;

    fs::create_dir_all(parent_dir).with_context(|| {
        format!(
            "Could not create directory for global config at '{}'.",
            parent_dir.display()
        )
    })?;

    let content = serde_json::to_string_pretty(config)
        .with_context(|| "Could not serialize global config.")?;

    // Atomic write: write to a temporary file and then rename
    let temp_path = config_path.with_file_name(format!(
        "{}.tmp",
        config_path.file_name().unwrap().to_string_lossy()
    ));
    // Ensure directory exists before writing temp file
    if let Some(parent) = temp_path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "Could not create directory for temporary global config file at '{}'.",
                parent.display()
            )
        })?;
    }

    fs::write(&temp_path, content).with_context(|| {
        format!(
            "Could not write temporary global config file to '{}'.",
            temp_path.display()
        )
    })?;

    fs::rename(&temp_path, &config_path).with_context(|| {
        format!(
            "Could not rename temporary global config file from '{}' to '{}'.",
            temp_path.display(),
            config_path.display()
        )
    })?;

    Ok(())
}

pub fn add_managed_project(
    project_name: &str,
    display_name: &str,
    source_url: &str,
    paths: &GupGlobalPaths,
) -> Result<()> {
    let mut config = load_global_config(paths)?;

    use crate::state_config::ManagedProjectInfo;
    let project_info = ManagedProjectInfo {
        unique_name: project_name.to_string(),
        display_name: display_name.to_string(),
        source_of_truth_url: source_url.to_string(),
        last_metadata_sync: None,
        preferred_default: None,
    };

    config
        .managed_projects
        .insert(project_name.to_string(), project_info);
    save_global_config(&config, paths)?;

    Ok(())
}

pub fn remove_managed_project(project_name: &str, paths: &GupGlobalPaths) -> Result<bool> {
    let mut config = load_global_config(paths)?;

    let removed = config.managed_projects.remove(project_name).is_some();

    if removed {
        save_global_config(&config, paths)?;
    }

    Ok(removed)
}

pub fn is_project_managed(project_name: &str, paths: &GupGlobalPaths) -> Result<bool> {
    let config = load_global_config(paths)?;
    Ok(config.managed_projects.contains_key(project_name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use tempfile::TempDir;

    // Test helper that creates a temporary directory and sets GUP_DEPOT_PATH
    struct TestEnvironment {
        _temp_dir: TempDir,
        original_env: Option<String>,
    }

    impl TestEnvironment {
        fn new() -> Result<Self> {
            let temp_dir = TempDir::new()?;

            // Save original environment variable
            let original_env = env::var("GUP_DEPOT_PATH").ok();

            // Set test environment to use temp directory
            env::set_var("GUP_DEPOT_PATH", temp_dir.path());

            Ok(TestEnvironment {
                _temp_dir: temp_dir,
                original_env,
            })
        }

        fn paths(&self) -> Result<crate::global_paths::GupGlobalPaths> {
            crate::global_paths::get_gup_paths()
        }
    }

    impl Drop for TestEnvironment {
        fn drop(&mut self) {
            // Restore original environment
            match &self.original_env {
                Some(val) => env::set_var("GUP_DEPOT_PATH", val),
                None => env::remove_var("GUP_DEPOT_PATH"),
            }
        }
    }

    #[test]
    fn test_load_default_config_when_file_missing() {
        let test_env = TestEnvironment::new().unwrap();
        let paths = test_env.paths().unwrap();
        let config = load_global_config(&paths).unwrap();

        // Verify default config properties
        assert!(config.managed_projects.is_empty());
        assert_eq!(config.self_update_channel, None);
        assert_eq!(config.background_self_update_interval_minutes, None);

        // Invariant: Default config should always be valid
        assert!(serde_json::to_string(&config).is_ok());
    }

    // Additional tests are in tests/config_management_test.rs
}
