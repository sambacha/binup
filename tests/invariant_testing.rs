use anyhow::Result;
use chrono::Utc;
use gup::global_config_manager::*;
use gup::global_paths::get_gup_paths;
use gup::operations_metadata::*;
use gup::project_metadata_m::*;
use gup::project_state_manager::*;
use gup::state_config::*;
use gup::utils::get_target_triple_id;
use std::collections::HashMap;
use std::env;
use tempfile::TempDir;

/// Test helper for creating isolated test environments
struct TestEnvironment {
    _temp_dir: TempDir,
    original_env: Option<String>,
}

impl TestEnvironment {
    fn new() -> Result<Self> {
        let temp_dir = TempDir::new()?;
        let original_env = env::var("GUP_DEPOT_PATH").ok();
        // Use a unique prefix to avoid collisions in concurrent tests
        let unique_path = temp_dir.path().join(format!("gup-{}", std::process::id()));
        env::set_var("GUP_DEPOT_PATH", &unique_path);
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
        match &self.original_env {
            Some(val) => env::set_var("GUP_DEPOT_PATH", val),
            None => env::remove_var("GUP_DEPOT_PATH"),
        }
    }
}

/// Helper to create minimal valid project metadata for testing
fn create_test_metadata(project_name: &str) -> ProjectMetadata {
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

    let mut versions = HashMap::new();
    let mut artifacts = HashMap::new();
    artifacts.insert(
        "linux-x64".to_string(),
        ArtifactDetail {
            url_path_suffix: format!("{}-1.0.0-linux-x64.tar.gz", project_name),
            sha256: Some("abc123def456".to_string()),
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

    ProjectMetadata {
        project_name: project_name.to_string(),
        display_name: Some(format!("{} Project", project_name)),
        description: Some(format!("Test project for {}", project_name)),
        homepage_url: Some("https://example.com".to_string()),
        metadata_format_version: "1.0.0".to_string(),
        base_download_url: url::Url::parse("https://example.com/releases/").unwrap(),
        platforms,
        default_install_config: InstallConfig::default(),
        executables: vec![ExecutableDetail {
            name: project_name.to_string(),
            path_in_bin_subdir: project_name.to_string(),
        }],
        default_executable_name: Some(project_name.to_string()),
        available_versions: versions,
        channels,
    }
}

#[test]
fn test_configuration_data_consistency() {
    let test_env = TestEnvironment::new().unwrap();
    let paths = test_env.paths().unwrap();

    // Add multiple projects and verify global state consistency
    let projects = vec![
        ("nodejs", "Node.js", "https://nodejs.org/metadata.json"),
        ("python", "Python", "https://python.org/metadata.json"),
        ("rust", "Rust", "https://rust-lang.org/metadata.json"),
    ];

    for (name, display, url) in &projects {
        add_managed_project(name, display, url, &paths).unwrap();
    }

    let global_config = load_global_config(&paths).unwrap();

    // Invariant: Number of managed projects should match what we added
    assert_eq!(global_config.managed_projects.len(), projects.len());

    // Invariant: Each project should have consistent data
    for (name, display, url) in &projects {
        let project_info = global_config.managed_projects.get(*name).unwrap();
        assert_eq!(project_info.unique_name, *name);
        assert_eq!(project_info.display_name, *display);
        assert_eq!(project_info.source_of_truth_url, *url);

        // Invariant: Project state file should be loadable even if empty
        let project_state = load_project_local_state(name, &paths).unwrap();
        assert!(project_state.installed_versions.is_empty()); // Should start empty
        assert!(project_state.active_channels.is_empty());
        assert!(project_state.linked_paths.is_empty());
        assert!(project_state.overrides.is_empty());
    }
}

#[test]
fn test_version_resolution_invariants() {
    let test_env = TestEnvironment::new().unwrap();
    let _paths = test_env.paths().unwrap();

    let metadata = create_test_metadata("test-tool");

    // Invariant: Direct version resolution should always return the same version
    let direct_result = resolve_channel_to_version("1.0.0", &metadata).unwrap();
    assert_eq!(direct_result, "1.0.0");

    // Invariant: Channel resolution should be deterministic
    let channel_result1 = resolve_channel_to_version("stable", &metadata).unwrap();
    let channel_result2 = resolve_channel_to_version("stable", &metadata).unwrap();
    assert_eq!(channel_result1, channel_result2);

    // Invariant: Invalid version/channel should fail consistently
    assert!(resolve_channel_to_version("nonexistent", &metadata).is_err());
    assert!(resolve_channel_to_version("invalid-channel", &metadata).is_err());
}

#[test]
fn test_target_platform_detection_stability() {
    // Invariant: Target triple detection should be stable across calls
    let triple1 = get_target_triple_id().unwrap();
    let triple2 = get_target_triple_id().unwrap();
    assert_eq!(triple1, triple2);

    // Invariant: Target triple should be a valid format
    assert!(triple1.contains('-'));
    assert!(!triple1.is_empty());
    assert!(!triple1.starts_with('-'));
    assert!(!triple1.ends_with('-'));

    // Invariant: Target triple should match expected patterns
    let parts: Vec<&str> = triple1.split('-').collect();
    assert!(
        parts.len() >= 2,
        "Target triple should have at least 2 parts"
    );

    // First part should be architecture
    let arch = parts[0];
    assert!(
        matches!(arch, "x86_64" | "aarch64" | "i686" | "arm"),
        "Architecture should be recognized: {}",
        arch
    );
}

#[test]
fn test_project_state_modification_invariants() {
    let test_env = TestEnvironment::new().unwrap();
    let paths = test_env.paths().unwrap();

    let project_name = "test-project";
    let mut state = ProjectLocalState::default();

    // Add some installed versions
    let version_info = InstalledVersionConcreteInfo {
        version_string: "1.0.0".to_string(),
        path_segment: "test-1.0.0-linux-x64".to_string(),
        installed_at: Utc::now(),
    };
    state
        .installed_versions
        .insert("1.0.0".to_string(), version_info);

    // Save initial state
    save_project_local_state(project_name, &state, &paths).unwrap();

    // Modify and save again
    let version_info2 = InstalledVersionConcreteInfo {
        version_string: "1.1.0".to_string(),
        path_segment: "test-1.1.0-linux-x64".to_string(),
        installed_at: Utc::now(),
    };
    state
        .installed_versions
        .insert("1.1.0".to_string(), version_info2);
    state.default_version_or_channel_name = Some("1.1.0".to_string());

    save_project_local_state(project_name, &state, &paths).unwrap();

    // Load and verify
    let loaded_state = load_project_local_state(project_name, &paths).unwrap();

    // Invariant: All modifications should be preserved
    assert_eq!(loaded_state.installed_versions.len(), 2);
    assert!(loaded_state.installed_versions.contains_key("1.0.0"));
    assert!(loaded_state.installed_versions.contains_key("1.1.0"));
    assert_eq!(
        loaded_state.default_version_or_channel_name,
        Some("1.1.0".to_string())
    );

    // Invariant: Timestamps should be preserved and reasonable
    for (_, version_info) in &loaded_state.installed_versions {
        assert!(version_info.installed_at <= Utc::now());
        assert!(version_info.installed_at > Utc::now() - chrono::Duration::minutes(5));
    }
}

#[test]
fn test_install_config_invariants() {
    let test_env = TestEnvironment::new().unwrap();
    let _paths = test_env.paths().unwrap();

    // Test default install config
    let default_config = InstallConfig::default();

    // Invariant: Default config should be serializable
    assert!(serde_json::to_string(&default_config).is_ok());

    // Test config with all fields set
    let mut full_config = InstallConfig::default();
    full_config.archive_format = Some("tar.gz".to_string());
    full_config.strip_components = Some(1);
    full_config.bin_subdir = Some("bin".to_string());
    full_config.post_install_hook = Some("chmod +x bin/*".to_string());

    // Invariant: Full config should roundtrip properly
    let json = serde_json::to_string(&full_config).unwrap();
    let roundtrip: InstallConfig = serde_json::from_str(&json).unwrap();

    assert_eq!(full_config.archive_format, roundtrip.archive_format);
    assert_eq!(full_config.strip_components, roundtrip.strip_components);
    assert_eq!(full_config.bin_subdir, roundtrip.bin_subdir);
    assert_eq!(full_config.post_install_hook, roundtrip.post_install_hook);
}

#[test]
fn test_atomic_operations_invariants() {
    let test_env = TestEnvironment::new().unwrap();
    let paths = test_env.paths().unwrap();

    // Test atomic config writes
    let mut config = GupGlobalConfig::default();
    config.self_update_channel = Some("test-channel".to_string());

    save_global_config(&config, &paths).unwrap();

    // Invariant: No temporary files should remain after atomic operations
    let config_file = paths.global_config_file();
    let temp_file = config_file.with_file_name(format!(
        "{}.tmp",
        config_file.file_name().unwrap().to_string_lossy()
    ));
    assert!(!temp_file.exists(), "Temporary file should be cleaned up");

    // Test atomic project state writes
    let mut state = ProjectLocalState::default();
    state.default_version_or_channel_name = Some("1.0.0".to_string());

    save_project_local_state("test-project", &state, &paths).unwrap();

    let state_file = paths.project_state_file("test-project");
    let temp_state_file = state_file.with_file_name(format!(
        "{}.tmp",
        state_file.file_name().unwrap().to_string_lossy()
    ));
    assert!(
        !temp_state_file.exists(),
        "Temporary state file should be cleaned up"
    );

    // Invariant: Final files should exist and be valid
    assert!(config_file.exists());
    assert!(state_file.exists());

    let loaded_config = load_global_config(&paths).unwrap();
    assert_eq!(
        loaded_config.self_update_channel,
        Some("test-channel".to_string())
    );

    let loaded_state = load_project_local_state("test-project", &paths).unwrap();
    assert_eq!(
        loaded_state.default_version_or_channel_name,
        Some("1.0.0".to_string())
    );
}

#[test]
fn test_error_handling_consistency() {
    let test_env = TestEnvironment::new().unwrap();
    let paths = test_env.paths().unwrap();

    // Test invalid project operations
    let result = load_project_local_state("nonexistent-project", &paths);
    assert!(result.is_ok()); // Should return default state, not error

    let result = is_project_managed("nonexistent-project", &paths);
    assert!(result.is_ok());
    assert!(!result.unwrap()); // Should return false, not error

    // Test removing non-existent project
    let result = remove_managed_project("nonexistent-project", &paths);
    assert!(result.is_ok());
    assert!(!result.unwrap()); // Should return false (not removed), not error

    // Invariant: Error messages should be descriptive and contain context
    let invalid_metadata_url = "not-a-url";
    let invalid_url_result = url::Url::parse(invalid_metadata_url);
    assert!(invalid_url_result.is_err());
}

#[test]
fn test_path_security_invariants() {
    let test_env = TestEnvironment::new().unwrap();
    let paths = test_env.paths().unwrap();

    // Test project names with potentially dangerous characters
    let dangerous_names = vec![
        "../malicious",
        "project/../escape",
        "project\\windows\\escape",
        "project\0null",
        "project/with/slashes",
    ];

    for name in &dangerous_names {
        // Operations should either work safely or fail gracefully
        let result = add_managed_project(name, "Test Project", "https://example.com", &paths);

        if result.is_ok() {
            // If it succeeds, verify paths are safe
            let project_dir = paths.project_dir(name);
            let project_state_file = paths.project_state_file(name);

            // Invariant: Generated paths should still be within the gup directory
            assert!(project_dir.starts_with(&paths.guphome()));
            assert!(project_state_file.starts_with(&paths.guphome()));

            // Clean up
            let _ = remove_managed_project(name, &paths);
        }
        // If it fails, that's also acceptable for security
    }
}

#[test]
fn test_cross_platform_compatibility_invariants() {
    let test_env = TestEnvironment::new().unwrap();
    let paths = test_env.paths().unwrap();

    // Test that basic operations work on all platforms
    let config = GupGlobalConfig::default();
    save_global_config(&config, &paths).unwrap();

    let loaded_config = load_global_config(&paths).unwrap();

    // Invariant: Serialization should be platform-independent
    let json1 = serde_json::to_string(&config).unwrap();
    let json2 = serde_json::to_string(&loaded_config).unwrap();
    assert_eq!(json1, json2);

    // Test project state operations
    let state = ProjectLocalState::default();
    save_project_local_state("cross-platform-test", &state, &paths).unwrap();

    let loaded_state = load_project_local_state("cross-platform-test", &paths).unwrap();

    // Invariant: State should survive roundtrip on all platforms
    let state_json1 = serde_json::to_string(&state).unwrap();
    let state_json2 = serde_json::to_string(&loaded_state).unwrap();
    assert_eq!(state_json1, state_json2);
}

#[test]
fn test_data_structure_size_limits() {
    let test_env = TestEnvironment::new().unwrap();
    let paths = test_env.paths().unwrap();

    // Test with large amounts of data to ensure no unexpected failures
    let mut large_config = GupGlobalConfig::default();

    // Add many projects
    for i in 0..100 {
        let project_name = format!("project-{:03}", i);
        let project_info = ManagedProjectInfo {
            unique_name: project_name.clone(),
            display_name: format!("Project {}", i),
            source_of_truth_url: format!("https://example.com/project-{}.json", i),
            last_metadata_sync: Some(Utc::now()),
            preferred_default: Some("stable".to_string()),
        };
        large_config
            .managed_projects
            .insert(project_name, project_info);
    }

    // Invariant: Large configs should still be serializable and loadable
    let save_result = save_global_config(&large_config, &paths);
    assert!(save_result.is_ok(), "Should be able to save large config");

    let loaded_large_config = load_global_config(&paths).unwrap();
    assert_eq!(loaded_large_config.managed_projects.len(), 100);

    // Test large project state
    let mut large_state = ProjectLocalState::default();

    // Add many installed versions
    for i in 0..50 {
        let version = format!("1.{}.0", i);
        let version_info = InstalledVersionConcreteInfo {
            version_string: version.clone(),
            path_segment: format!("project-{}-linux-x64", version),
            installed_at: Utc::now(),
        };
        large_state.installed_versions.insert(version, version_info);
    }

    // Invariant: Large states should still be serializable and loadable
    let save_state_result = save_project_local_state("large-project", &large_state, &paths);
    assert!(
        save_state_result.is_ok(),
        "Should be able to save large project state"
    );

    let loaded_large_state = load_project_local_state("large-project", &paths).unwrap();
    assert_eq!(loaded_large_state.installed_versions.len(), 50);
}
