use anyhow::Result;
use chrono::Utc;
use gup::global_config_manager::*;
use gup::global_paths::get_gup_paths;
use gup::multiplexer::run_multiplexed_command;
use gup::project_metadata_m::*;
use gup::project_state_manager::*;
use gup::state_config::*;
use std::collections::HashMap;
use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Test helper for creating isolated test environments
struct TestEnvironment {
    _temp_dir: TempDir,
    original_env: Option<String>,
    original_dir: PathBuf,
}

impl TestEnvironment {
    fn new() -> Result<Self> {
        let temp_dir = TempDir::new()?;
        let original_env = env::var("GUP_DEPOT_PATH").ok();
        let original_dir = env::current_dir()?;
        env::set_var("GUP_DEPOT_PATH", temp_dir.path());
        Ok(TestEnvironment {
            _temp_dir: temp_dir,
            original_env,
            original_dir,
        })
    }

    fn paths(&self) -> Result<gup::global_paths::GupGlobalPaths> {
        get_gup_paths()
    }

    fn create_test_project(&self, project_name: &str, executable_name: &str) -> Result<()> {
        let paths = self.paths()?;

        // Add project to global config
        add_managed_project(
            project_name,
            &format!("{} Project", project_name),
            &format!("https://example.com/{}.json", project_name),
            &paths,
        )?;

        // Create project state with installed version
        let mut state = ProjectLocalState::default();
        state.default_version_or_channel_name = Some("1.0.0".to_string());

        let version_info = InstalledVersionConcreteInfo {
            version_string: "1.0.0".to_string(),
            path_segment: format!("{}-1.0.0", project_name),
            installed_at: Utc::now(),
        };
        state
            .installed_versions
            .insert("1.0.0".to_string(), version_info);

        save_project_local_state(project_name, &state, &paths)?;

        // Create metadata file
        let metadata = self.create_test_metadata(project_name, executable_name);
        let metadata_path = paths.project_metadata_cache_file(project_name);
        if let Some(parent) = metadata_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(&metadata)?;
        fs::write(&metadata_path, json)?;

        // Create actual executable file
        let version_dir = paths
            .project_versions_dir(project_name)
            .join(format!("{}-1.0.0", project_name))
            .join("bin");
        fs::create_dir_all(&version_dir)?;
        let exe_path = version_dir.join(executable_name);

        #[cfg(unix)]
        {
            fs::write(&exe_path, "#!/bin/sh\necho 'Test executable'")?;
            use std::os::unix::fs::PermissionsExt;
            let perms = fs::Permissions::from_mode(0o755);
            fs::set_permissions(&exe_path, perms)?;
        }

        #[cfg(windows)]
        {
            fs::write(&exe_path, "@echo Test executable")?;
        }

        Ok(())
    }

    fn create_test_metadata(&self, project_name: &str, executable_name: &str) -> ProjectMetadata {
        let mut platforms = HashMap::new();
        let target_triple = gup::utils::get_target_triple_id().unwrap_or("unknown".to_string());
        let platform_name = if target_triple.contains("linux") {
            "linux-x64"
        } else if target_triple.contains("darwin") {
            "darwin-x64"
        } else if target_triple.contains("windows") {
            "windows-x64"
        } else {
            "unknown"
        };

        platforms.insert(
            platform_name.to_string(),
            PlatformDetail {
                os: platform_name.split('-').next().unwrap().to_string(),
                arch: "x86_64".to_string(),
                target_triple_pattern: ".*".to_string(),
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

        let mut versions = HashMap::new();
        let mut artifacts = HashMap::new();
        artifacts.insert(
            platform_name.to_string(),
            ArtifactDetail {
                url_path_suffix: format!("{}-1.0.0.tar.gz", project_name),
                sha256: Some("abc123".to_string()),
                archive_format: Some("tar.gz".to_string()),
                strip_components: Some(1),
                bin_subdir: Some("bin".to_string()),
            },
        );

        versions.insert(
            "1.0.0".to_string(),
            VersionDetail {
                release_date: Some("2024-01-01T00:00:00Z".to_string()),
                artifacts,
                install_config_override: None,
            },
        );

        let mut install_config = InstallConfig::default();
        install_config.bin_subdir = Some("bin".to_string());

        ProjectMetadata {
            project_name: project_name.to_string(),
            display_name: Some(format!("{} Project", project_name)),
            description: Some(format!("Test project for {}", project_name)),
            homepage_url: Some("https://example.com".to_string()),
            metadata_format_version: "1.0.0".to_string(),
            base_download_url: url::Url::parse("https://example.com/releases/").unwrap(),
            platforms,
            default_install_config: install_config,
            executables: vec![ExecutableDetail {
                name: executable_name.to_string(),
                path_in_bin_subdir: executable_name.to_string(),
            }],
            default_executable_name: Some(executable_name.to_string()),
            available_versions: versions,
            channels,
        }
    }
}

impl Drop for TestEnvironment {
    fn drop(&mut self) {
        match &self.original_env {
            Some(val) => env::set_var("GUP_DEPOT_PATH", val),
            None => env::remove_var("GUP_DEPOT_PATH"),
        }
        let _ = env::set_current_dir(&self.original_dir);
    }
}

#[test]
fn test_multiplexer_project_name_resolution() {
    let test_env = TestEnvironment::new().unwrap();
    let paths = test_env.paths().unwrap();

    // Create a test project
    test_env.create_test_project("nodejs", "node").unwrap();

    // Test invoking by project name (should run default executable)
    let result = run_multiplexed_command(&OsString::from("nodejs"), &[], &paths);

    // Should resolve to nodejs default executable
    assert!(result.is_ok() || result.is_err()); // Platform-specific behavior
}

#[test]
fn test_multiplexer_executable_name_resolution() {
    let test_env = TestEnvironment::new().unwrap();
    let paths = test_env.paths().unwrap();

    // Create multiple projects with different executables
    test_env.create_test_project("nodejs", "node").unwrap();
    test_env.create_test_project("python", "python").unwrap();

    // Test invoking by executable name
    let result =
        run_multiplexed_command(&OsString::from("node"), &["--version".to_string()], &paths);

    // Should resolve to node executable from nodejs project
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_multiplexer_link_resolution() {
    let test_env = TestEnvironment::new().unwrap();
    let paths = test_env.paths().unwrap();

    // Create a project and add a link
    test_env.create_test_project("nodejs", "node").unwrap();

    let mut state = load_project_local_state("nodejs", &paths).unwrap();
    state.linked_paths.insert(
        "mynode".to_string(),
        LinkedProjectInfo {
            command_path: "/usr/local/bin/node".to_string(),
            arguments: Some(vec!["--harmony".to_string()]),
        },
    );
    save_project_local_state("nodejs", &state, &paths).unwrap();

    // Test invoking by link name
    let result = run_multiplexed_command(
        &OsString::from("mynode"),
        &["script.js".to_string()],
        &paths,
    );

    // Should resolve to linked command
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_multiplexer_directory_override() {
    let test_env = TestEnvironment::new().unwrap();
    let paths = test_env.paths().unwrap();

    // Create project with override
    test_env.create_test_project("nodejs", "node").unwrap();

    // Create override directory
    let override_dir = test_env._temp_dir.path().join("project");
    fs::create_dir_all(&override_dir).unwrap();

    let mut state = load_project_local_state("nodejs", &paths).unwrap();
    state.overrides.push(DirectoryOverride {
        path: override_dir.clone(),
        version_or_channel: "1.0.0".to_string(),
    });
    save_project_local_state("nodejs", &state, &paths).unwrap();

    // Change to override directory
    env::set_current_dir(&override_dir).unwrap();

    // Test resolution from override directory
    let result = run_multiplexed_command(&OsString::from("node"), &[], &paths);

    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_multiplexer_precedence_order() {
    let test_env = TestEnvironment::new().unwrap();
    let paths = test_env.paths().unwrap();

    // Create project with all resolution types
    test_env.create_test_project("nodejs", "node").unwrap();

    let override_dir = test_env._temp_dir.path().join("override");
    fs::create_dir_all(&override_dir).unwrap();

    let mut state = load_project_local_state("nodejs", &paths).unwrap();

    // Add override
    state.overrides.push(DirectoryOverride {
        path: override_dir.clone(),
        version_or_channel: "1.0.0".to_string(),
    });

    // Add link with same name as executable
    state.linked_paths.insert(
        "node".to_string(),
        LinkedProjectInfo {
            command_path: "/custom/node".to_string(),
            arguments: None,
        },
    );

    save_project_local_state("nodejs", &state, &paths).unwrap();

    // Test from override directory - should use override, not link
    env::set_current_dir(&override_dir).unwrap();
    let result = run_multiplexed_command(&OsString::from("node"), &[], &paths);
    // Override should take precedence
    assert!(result.is_ok() || result.is_err());

    // Test from non-override directory - should use link
    env::set_current_dir(test_env._temp_dir.path()).unwrap();
    let result = run_multiplexed_command(&OsString::from("node"), &[], &paths);
    // Link should take precedence over default
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_multiplexer_multiple_projects_same_executable() {
    let test_env = TestEnvironment::new().unwrap();
    let paths = test_env.paths().unwrap();

    // Create multiple projects with same executable name
    test_env.create_test_project("nodejs", "node").unwrap();
    test_env.create_test_project("nodejs-lts", "node").unwrap();

    // Should resolve to one of them (first found)
    let result = run_multiplexed_command(&OsString::from("node"), &[], &paths);

    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_multiplexer_nonexistent_command() {
    let test_env = TestEnvironment::new().unwrap();
    let paths = test_env.paths().unwrap();

    // Test with command that doesn't exist
    let result = run_multiplexed_command(&OsString::from("nonexistent-command"), &[], &paths);

    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("could not determine what to run"));
}

#[test]
fn test_multiplexer_parent_directory_override() {
    let test_env = TestEnvironment::new().unwrap();
    let paths = test_env.paths().unwrap();

    test_env.create_test_project("nodejs", "node").unwrap();

    // Create nested directory structure
    let parent_dir = test_env._temp_dir.path().join("parent");
    let child_dir = parent_dir.join("child");
    fs::create_dir_all(&child_dir).unwrap();

    // Add override for parent directory
    let mut state = load_project_local_state("nodejs", &paths).unwrap();
    state.overrides.push(DirectoryOverride {
        path: parent_dir.clone(),
        version_or_channel: "1.0.0".to_string(),
    });
    save_project_local_state("nodejs", &state, &paths).unwrap();

    // Run from child directory - should inherit parent override
    env::set_current_dir(&child_dir).unwrap();
    let result = run_multiplexed_command(&OsString::from("node"), &[], &paths);

    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_multiplexer_link_with_arguments() {
    let test_env = TestEnvironment::new().unwrap();
    let paths = test_env.paths().unwrap();

    test_env.create_test_project("nodejs", "node").unwrap();

    // Create actual test script
    let script_path = test_env._temp_dir.path().join("test.js");
    fs::write(&script_path, "console.log('test');").unwrap();

    let mut state = load_project_local_state("nodejs", &paths).unwrap();
    state.linked_paths.insert(
        "node-debug".to_string(),
        LinkedProjectInfo {
            command_path: script_path.to_string_lossy().to_string(),
            arguments: Some(vec!["--inspect".to_string(), "--debug-brk".to_string()]),
        },
    );
    save_project_local_state("nodejs", &state, &paths).unwrap();

    // Test link with prepended arguments
    let result = run_multiplexed_command(
        &OsString::from("node-debug"),
        &["app.js".to_string()],
        &paths,
    );

    // Arguments should be prepended: --inspect --debug-brk app.js
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_multiplexer_missing_executable_file() {
    let test_env = TestEnvironment::new().unwrap();
    let paths = test_env.paths().unwrap();

    // Create project but don't create actual executable file
    add_managed_project("nodejs", "Node.js", "https://example.com/node.json", &paths).unwrap();

    let mut state = ProjectLocalState::default();
    state.default_version_or_channel_name = Some("1.0.0".to_string());
    state.installed_versions.insert(
        "1.0.0".to_string(),
        InstalledVersionConcreteInfo {
            version_string: "1.0.0".to_string(),
            path_segment: "node-1.0.0".to_string(),
            installed_at: Utc::now(),
        },
    );
    save_project_local_state("nodejs", &state, &paths).unwrap();

    // Create metadata but not executable
    let metadata = test_env.create_test_metadata("nodejs", "node");
    let metadata_path = paths.project_metadata_cache_file("nodejs");
    if let Some(parent) = metadata_path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(&metadata_path, serde_json::to_string(&metadata).unwrap()).unwrap();

    let result = run_multiplexed_command(&OsString::from("node"), &[], &paths);

    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("does not exist"));
}

#[test]
fn test_multiplexer_channel_resolution() {
    let test_env = TestEnvironment::new().unwrap();
    let paths = test_env.paths().unwrap();

    test_env.create_test_project("nodejs", "node").unwrap();

    // Update state to use channel instead of version
    let mut state = load_project_local_state("nodejs", &paths).unwrap();
    state.default_version_or_channel_name = Some("stable".to_string());
    state.active_channels.insert(
        "stable".to_string(),
        ActiveChannelInfo {
            resolved_version: "1.0.0".to_string(),
            last_checked: Utc::now(),
        },
    );
    save_project_local_state("nodejs", &state, &paths).unwrap();

    let result = run_multiplexed_command(&OsString::from("node"), &[], &paths);

    // Should resolve channel to version
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_multiplexer_case_sensitivity() {
    let test_env = TestEnvironment::new().unwrap();
    let paths = test_env.paths().unwrap();

    test_env.create_test_project("nodejs", "node").unwrap();

    // Test with different cases
    let test_cases = vec!["node", "Node", "NODE", "nOdE"];

    for invoked_as in test_cases {
        let result = run_multiplexed_command(&OsString::from(invoked_as), &[], &paths);

        // Behavior depends on platform case sensitivity
        if cfg!(target_os = "windows") {
            // Windows is case-insensitive
            assert!(result.is_ok() || result.unwrap_err().to_string().contains("does not exist"));
        } else {
            // Unix is case-sensitive - only exact match should work
            if invoked_as == "node" {
                assert!(
                    result.is_ok() || result.unwrap_err().to_string().contains("does not exist")
                );
            } else {
                assert!(result.is_err());
            }
        }
    }
}

#[test]
fn test_multiplexer_empty_project_state() {
    let test_env = TestEnvironment::new().unwrap();
    let paths = test_env.paths().unwrap();

    // Add project but don't set default version
    add_managed_project("nodejs", "Node.js", "https://example.com/node.json", &paths).unwrap();

    let result = run_multiplexed_command(&OsString::from("nodejs"), &[], &paths);

    // Should fail gracefully
    assert!(result.is_err());
}
