use anyhow::Result;
use chrono::Utc;
use gup::global_paths::get_gup_paths;
use gup::project_state_manager::*;
use gup::state_config::{
    ActiveChannelInfo, DirectoryOverride, InstalledVersionConcreteInfo, LinkedProjectInfo,
    ProjectLocalState,
};
use std::env;
use std::fs;
use std::path::PathBuf;
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

    fn paths(&self) -> Result<gup::global_paths::GupGlobalPaths> {
        get_gup_paths()
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
fn test_default_project_state() {
    let test_env = TestEnvironment::new().unwrap();
    let paths = test_env.paths().unwrap();

    // Loading non-existent project should return default state
    let state = load_project_local_state("nonexistent-project", &paths).unwrap();

    // Verify default state properties
    assert!(state.default_version_or_channel_name.is_none());
    assert!(state.installed_versions.is_empty());
    assert!(state.active_channels.is_empty());
    assert!(state.linked_paths.is_empty());
    assert!(state.overrides.is_empty());

    // Invariant: Default state should be serializable
    assert!(serde_json::to_string(&state).is_ok());
}

#[test]
fn test_project_state_roundtrip() {
    let test_env = TestEnvironment::new().unwrap();
    let paths = test_env.paths().unwrap();

    // Create comprehensive project state
    let mut original_state = ProjectLocalState::default();
    original_state.default_version_or_channel_name = Some("stable".to_string());

    // Add installed versions
    let version_info = InstalledVersionConcreteInfo {
        version_string: "1.0.0".to_string(),
        path_segment: "nodejs-1.0.0-linux-x64".to_string(),
        installed_at: Utc::now(),
    };
    original_state
        .installed_versions
        .insert("1.0.0".to_string(), version_info);

    // Add active channels
    let channel_info = ActiveChannelInfo {
        resolved_version: "1.0.0".to_string(),
        last_checked: Utc::now(),
    };
    original_state
        .active_channels
        .insert("stable".to_string(), channel_info);

    // Add linked paths
    let link_info = LinkedProjectInfo {
        command_path: "/usr/local/bin/nodejs-dev".to_string(),
        arguments: Some(vec!["--dev".to_string(), "--verbose".to_string()]),
    };
    original_state
        .linked_paths
        .insert("dev".to_string(), link_info);

    // Add directory overrides
    let override_info = DirectoryOverride {
        path: PathBuf::from("/home/user/project"),
        version_or_channel: "1.0.0".to_string(),
    };
    original_state.overrides.push(override_info);

    // Save and reload
    save_project_local_state("test-project", &original_state, &paths).unwrap();
    let loaded_state = load_project_local_state("test-project", &paths).unwrap();

    // Verify all fields preserved
    assert_eq!(
        loaded_state.default_version_or_channel_name,
        original_state.default_version_or_channel_name
    );
    assert_eq!(loaded_state.installed_versions.len(), 1);
    assert_eq!(loaded_state.active_channels.len(), 1);
    assert_eq!(loaded_state.linked_paths.len(), 1);
    assert_eq!(loaded_state.overrides.len(), 1);

    // Verify detailed content
    let loaded_version = loaded_state.installed_versions.get("1.0.0").unwrap();
    let original_version = original_state.installed_versions.get("1.0.0").unwrap();
    assert_eq!(
        loaded_version.version_string,
        original_version.version_string
    );
    assert_eq!(loaded_version.path_segment, original_version.path_segment);

    let loaded_channel = loaded_state.active_channels.get("stable").unwrap();
    let original_channel = original_state.active_channels.get("stable").unwrap();
    assert_eq!(
        loaded_channel.resolved_version,
        original_channel.resolved_version
    );

    let loaded_link = loaded_state.linked_paths.get("dev").unwrap();
    let original_link = original_state.linked_paths.get("dev").unwrap();
    assert_eq!(loaded_link.command_path, original_link.command_path);
    assert_eq!(loaded_link.arguments, original_link.arguments);

    let loaded_override = &loaded_state.overrides[0];
    let original_override = &original_state.overrides[0];
    assert_eq!(loaded_override.path, original_override.path);
    assert_eq!(
        loaded_override.version_or_channel,
        original_override.version_or_channel
    );
}

#[test]
fn test_multiple_installed_versions() {
    let test_env = TestEnvironment::new().unwrap();
    let paths = test_env.paths().unwrap();

    let mut state = ProjectLocalState::default();

    // Add multiple versions with different install times
    let versions = vec![
        ("1.0.0", "nodejs-1.0.0-linux-x64"),
        ("1.1.0", "nodejs-1.1.0-linux-x64"),
        ("2.0.0-beta", "nodejs-2.0.0-beta-linux-x64"),
    ];

    for (version, path_segment) in &versions {
        let version_info = InstalledVersionConcreteInfo {
            version_string: version.to_string(),
            path_segment: path_segment.to_string(),
            installed_at: Utc::now(),
        };
        state
            .installed_versions
            .insert(version.to_string(), version_info);
    }

    save_project_local_state("multi-version-project", &state, &paths).unwrap();
    let loaded_state = load_project_local_state("multi-version-project", &paths).unwrap();

    // Verify all versions preserved
    assert_eq!(loaded_state.installed_versions.len(), 3);

    for (version, expected_path) in &versions {
        let version_info = loaded_state.installed_versions.get(*version).unwrap();
        assert_eq!(version_info.version_string, *version);
        assert_eq!(version_info.path_segment, *expected_path);

        // Invariant: All installed versions should have valid timestamps
        assert!(version_info.installed_at <= Utc::now());
    }
}

#[test]
fn test_channel_tracking() {
    let test_env = TestEnvironment::new().unwrap();
    let paths = test_env.paths().unwrap();

    let mut state = ProjectLocalState::default();

    // Add multiple channels
    let channels = vec![
        ("stable", "1.0.0"),
        ("beta", "1.1.0-beta"),
        ("nightly", "1.2.0-nightly.20241201"),
    ];

    for (channel, resolved_version) in &channels {
        let channel_info = ActiveChannelInfo {
            resolved_version: resolved_version.to_string(),
            last_checked: Utc::now(),
        };
        state
            .active_channels
            .insert(channel.to_string(), channel_info);
    }

    save_project_local_state("channel-tracking-project", &state, &paths).unwrap();
    let loaded_state = load_project_local_state("channel-tracking-project", &paths).unwrap();

    // Verify all channels preserved
    assert_eq!(loaded_state.active_channels.len(), 3);

    for (channel, expected_version) in &channels {
        let channel_info = loaded_state.active_channels.get(*channel).unwrap();
        assert_eq!(channel_info.resolved_version, *expected_version);

        // Invariant: Channel check times should be reasonable
        assert!(channel_info.last_checked <= Utc::now());
    }
}

#[test]
fn test_linked_paths_management() {
    let test_env = TestEnvironment::new().unwrap();
    let paths = test_env.paths().unwrap();

    let mut state = ProjectLocalState::default();

    // Add various types of linked paths
    let links = vec![
        (
            "dev",
            "/usr/local/bin/nodejs-dev",
            Some(vec!["--dev".to_string()]),
        ),
        ("production", "/opt/nodejs/bin/node", None),
        (
            "debug",
            "/home/user/builds/node",
            Some(vec!["--inspect".to_string(), "--debug-brk".to_string()]),
        ),
    ];

    for (name, path, args) in &links {
        let link_info = LinkedProjectInfo {
            command_path: path.to_string(),
            arguments: args.clone(),
        };
        state.linked_paths.insert(name.to_string(), link_info);
    }

    save_project_local_state("linked-project", &state, &paths).unwrap();
    let loaded_state = load_project_local_state("linked-project", &paths).unwrap();

    // Verify all links preserved
    assert_eq!(loaded_state.linked_paths.len(), 3);

    for (name, expected_path, expected_args) in &links {
        let link_info = loaded_state.linked_paths.get(*name).unwrap();
        assert_eq!(link_info.command_path, *expected_path);
        assert_eq!(link_info.arguments, *expected_args);
    }
}

#[test]
fn test_directory_overrides() {
    let test_env = TestEnvironment::new().unwrap();
    let paths = test_env.paths().unwrap();

    let mut state = ProjectLocalState::default();

    // Add directory overrides for different paths
    let overrides = vec![
        ("/home/user/project1", "1.0.0"),
        ("/home/user/project2", "stable"),
        ("/tmp/test-project", "beta"),
    ];

    for (path, version) in &overrides {
        let override_info = DirectoryOverride {
            path: PathBuf::from(*path),
            version_or_channel: version.to_string(),
        };
        state.overrides.push(override_info);
    }

    save_project_local_state("override-project", &state, &paths).unwrap();
    let loaded_state = load_project_local_state("override-project", &paths).unwrap();

    // Verify all overrides preserved
    assert_eq!(loaded_state.overrides.len(), 3);

    for (i, (expected_path, expected_version)) in overrides.iter().enumerate() {
        let override_info = &loaded_state.overrides[i];
        assert_eq!(override_info.path, PathBuf::from(*expected_path));
        assert_eq!(override_info.version_or_channel, *expected_version);
    }
}

#[test]
fn test_state_file_atomic_writes() {
    let test_env = TestEnvironment::new().unwrap();
    let paths = test_env.paths().unwrap();

    let mut state = ProjectLocalState::default();
    state.default_version_or_channel_name = Some("test-version".to_string());

    save_project_local_state("atomic-test-project", &state, &paths).unwrap();

    // Check that temporary file is cleaned up
    let state_file = paths.project_state_file("atomic-test-project");
    let temp_file = state_file.with_extension("json.tmp");
    assert!(!temp_file.exists(), "Temporary file should be cleaned up");

    // Verify final file exists and is valid
    assert!(state_file.exists());
    let content = fs::read_to_string(&state_file).unwrap();
    let _: ProjectLocalState = serde_json::from_str(&content).unwrap();
}

#[test]
fn test_invalid_state_file_handling() {
    let test_env = TestEnvironment::new().unwrap();
    let paths = test_env.paths().unwrap();

    let state_file = paths.project_state_file("invalid-project");

    // Create parent directory
    if let Some(parent) = state_file.parent() {
        fs::create_dir_all(parent).unwrap();
    }

    // Write invalid JSON
    fs::write(&state_file, "{ invalid json syntax }").unwrap();

    // Should return error when trying to load
    let result = load_project_local_state("invalid-project", &paths);
    assert!(result.is_err());

    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("Could not parse project state file"));
}

#[test]
fn test_concurrent_state_operations() {
    let test_env = TestEnvironment::new().unwrap();
    let paths = test_env.paths().unwrap();

    // Simulate concurrent operations on same project
    let mut state1 = ProjectLocalState::default();
    state1.default_version_or_channel_name = Some("1.0.0".to_string());

    let version_info = InstalledVersionConcreteInfo {
        version_string: "1.0.0".to_string(),
        path_segment: "test-1.0.0".to_string(),
        installed_at: Utc::now(),
    };
    state1
        .installed_versions
        .insert("1.0.0".to_string(), version_info);

    save_project_local_state("concurrent-project", &state1, &paths).unwrap();

    // Load, modify, and save again
    let mut state2 = load_project_local_state("concurrent-project", &paths).unwrap();

    let another_version = InstalledVersionConcreteInfo {
        version_string: "1.1.0".to_string(),
        path_segment: "test-1.1.0".to_string(),
        installed_at: Utc::now(),
    };
    state2
        .installed_versions
        .insert("1.1.0".to_string(), another_version);
    state2.default_version_or_channel_name = Some("1.1.0".to_string());

    save_project_local_state("concurrent-project", &state2, &paths).unwrap();

    // Verify final state
    let final_state = load_project_local_state("concurrent-project", &paths).unwrap();
    assert_eq!(
        final_state.default_version_or_channel_name,
        Some("1.1.0".to_string())
    );
    assert_eq!(final_state.installed_versions.len(), 2);
    assert!(final_state.installed_versions.contains_key("1.0.0"));
    assert!(final_state.installed_versions.contains_key("1.1.0"));
}

#[test]
fn test_state_serialization_stability() {
    let test_env = TestEnvironment::new().unwrap();
    let _paths = test_env.paths().unwrap();

    // Create state with all possible field types
    let mut state = ProjectLocalState::default();
    state.default_version_or_channel_name = Some("stable".to_string());

    // Add complex data structures
    let version_info = InstalledVersionConcreteInfo {
        version_string: "1.0.0".to_string(),
        path_segment: "complex-1.0.0-linux-x64".to_string(),
        installed_at: Utc::now(),
    };
    state
        .installed_versions
        .insert("1.0.0".to_string(), version_info);

    let channel_info = ActiveChannelInfo {
        resolved_version: "1.0.0".to_string(),
        last_checked: Utc::now(),
    };
    state
        .active_channels
        .insert("stable".to_string(), channel_info);

    // Multiple serializations should be identical
    let json1 = serde_json::to_string_pretty(&state).unwrap();
    let json2 = serde_json::to_string_pretty(&state).unwrap();
    assert_eq!(json1, json2);

    // Roundtrip should preserve everything
    let deserialized: ProjectLocalState = serde_json::from_str(&json1).unwrap();
    let json3 = serde_json::to_string_pretty(&deserialized).unwrap();
    assert_eq!(json1, json3);

    // Invariant: All data should be preserved
    assert_eq!(
        state.default_version_or_channel_name,
        deserialized.default_version_or_channel_name
    );
    assert_eq!(
        state.installed_versions.len(),
        deserialized.installed_versions.len()
    );
    assert_eq!(
        state.active_channels.len(),
        deserialized.active_channels.len()
    );
}

#[test]
fn test_edge_case_project_names() {
    let test_env = TestEnvironment::new().unwrap();
    let paths = test_env.paths().unwrap();

    let edge_case_names = vec![
        "", // Empty name
        "project-with-many-dashes",
        "project_with_underscores",
        "project.with.dots",
        "project@with@symbols",
        "very-long-project-name-that-exceeds-normal-expectations-but-should-still-work",
        "项目中文名称",      // Unicode
        "проект-на-русском", // Cyrillic
    ];

    for name in &edge_case_names {
        let mut state = ProjectLocalState::default();
        state.default_version_or_channel_name = Some("1.0.0".to_string());

        // Should handle all edge case names
        save_project_local_state(name, &state, &paths).unwrap();
        let loaded_state = load_project_local_state(name, &paths).unwrap();

        assert_eq!(
            loaded_state.default_version_or_channel_name,
            Some("1.0.0".to_string())
        );
    }
}
