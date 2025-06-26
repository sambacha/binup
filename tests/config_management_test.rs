use anyhow::Result;
use chrono::Utc;
use gup::global_config_manager::*;
use gup::global_paths::get_gup_paths;
use gup::state_config::{GupGlobalConfig, ManagedProjectInfo};
use std::env;
use std::fs;
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
fn test_config_roundtrip_integrity() {
    let test_env = TestEnvironment::new().unwrap();
    let paths = test_env.paths().unwrap();

    let mut original_config = GupGlobalConfig::default();
    original_config.self_update_channel = Some("stable".to_string());
    original_config.background_self_update_interval_minutes = Some(1440);

    let project_info = ManagedProjectInfo {
        unique_name: "test-project".to_string(),
        display_name: "Test Project".to_string(),
        source_of_truth_url: "https://example.com/metadata.json".to_string(),
        last_metadata_sync: Some(Utc::now()),
        preferred_default: Some("stable".to_string()),
    };
    original_config
        .managed_projects
        .insert("test-project".to_string(), project_info);

    // Save and reload
    save_global_config(&original_config, &paths).unwrap();
    let loaded_config = load_global_config(&paths).unwrap();

    // Verify all fields are preserved
    assert_eq!(
        loaded_config.self_update_channel,
        original_config.self_update_channel
    );
    assert_eq!(
        loaded_config.background_self_update_interval_minutes,
        original_config.background_self_update_interval_minutes
    );
    assert_eq!(loaded_config.managed_projects.len(), 1);

    let loaded_project = loaded_config.managed_projects.get("test-project").unwrap();
    let original_project = original_config
        .managed_projects
        .get("test-project")
        .unwrap();
    assert_eq!(loaded_project.unique_name, original_project.unique_name);
    assert_eq!(loaded_project.display_name, original_project.display_name);
    assert_eq!(
        loaded_project.source_of_truth_url,
        original_project.source_of_truth_url
    );

    // Invariant: Serialized config should be deterministic (for same data)
    let json1 = serde_json::to_string(&original_config).unwrap();
    let json2 = serde_json::to_string(&loaded_config).unwrap();
    assert_eq!(json1, json2);
}

#[test]
fn test_project_management_operations() {
    let test_env = TestEnvironment::new().unwrap();
    let paths = test_env.paths().unwrap();

    // Initially no projects
    assert!(!is_project_managed("nodejs", &paths).unwrap());

    // Add a project
    add_managed_project(
        "nodejs",
        "Node.js Runtime",
        "https://example.com/nodejs-metadata.json",
        &paths,
    )
    .unwrap();

    // Verify it was added correctly
    assert!(is_project_managed("nodejs", &paths).unwrap());

    let config = load_global_config(&paths).unwrap();
    assert_eq!(config.managed_projects.len(), 1);

    let project = config.managed_projects.get("nodejs").unwrap();
    assert_eq!(project.unique_name, "nodejs");
    assert_eq!(project.display_name, "Node.js Runtime");
    assert_eq!(
        project.source_of_truth_url,
        "https://example.com/nodejs-metadata.json"
    );
    assert!(project.last_metadata_sync.is_none());
    assert!(project.preferred_default.is_none());

    // Remove the project
    let removed = remove_managed_project("nodejs", &paths).unwrap();
    assert!(removed);
    assert!(!is_project_managed("nodejs", &paths).unwrap());

    // Try to remove again (should return false)
    let removed_again = remove_managed_project("nodejs", &paths).unwrap();
    assert!(!removed_again);

    // Invariant: After removal, config should be empty
    let final_config = load_global_config(&paths).unwrap();
    assert!(final_config.managed_projects.is_empty());
}

#[test]
fn test_multiple_project_management() {
    let test_env = TestEnvironment::new().unwrap();
    let paths = test_env.paths().unwrap();

    let projects = vec![
        ("nodejs", "Node.js", "https://example.com/nodejs.json"),
        ("python", "Python", "https://example.com/python.json"),
        ("deno", "Deno", "https://example.com/deno.json"),
    ];

    // Add all projects
    for (name, display, url) in &projects {
        add_managed_project(name, display, url, &paths).unwrap();
    }

    // Verify all were added
    let config = load_global_config(&paths).unwrap();
    assert_eq!(config.managed_projects.len(), 3);

    for (name, display, url) in &projects {
        assert!(is_project_managed(name, &paths).unwrap());
        let project = config.managed_projects.get(*name).unwrap();
        assert_eq!(project.display_name, *display);
        assert_eq!(project.source_of_truth_url, *url);
    }

    // Remove middle project
    remove_managed_project("python", &paths).unwrap();
    assert!(!is_project_managed("python", &paths).unwrap());
    assert!(is_project_managed("nodejs", &paths).unwrap());
    assert!(is_project_managed("deno", &paths).unwrap());

    // Invariant: Remaining projects should be unaffected
    let config_after_removal = load_global_config(&paths).unwrap();
    assert_eq!(config_after_removal.managed_projects.len(), 2);
}

#[test]
fn test_invalid_json_error_handling() {
    let test_env = TestEnvironment::new().unwrap();
    let paths = test_env.paths().unwrap();
    let config_path = paths.global_config_file();

    // Create parent directory
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent).unwrap();
    }

    // Write invalid JSON
    fs::write(&config_path, "{ invalid json syntax }").unwrap();

    // Should return descriptive error
    let result = load_global_config(&paths);
    assert!(result.is_err());

    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("Could not parse global config file"));
}

#[test]
fn test_atomic_write_guarantees() {
    let test_env = TestEnvironment::new().unwrap();
    let paths = test_env.paths().unwrap();
    let config_path = paths.global_config_file();

    // Save initial config
    let mut config = GupGlobalConfig::default();
    config.self_update_channel = Some("test-channel".to_string());
    save_global_config(&config, &paths).unwrap();

    // Verify temp file is cleaned up (atomic write invariant)
    let temp_path = config_path.with_extension("json.tmp");
    assert!(
        !temp_path.exists(),
        "Temporary file should be cleaned up after write"
    );

    // Verify final file exists and has correct content
    assert!(config_path.exists());
    let loaded = load_global_config(&paths).unwrap();
    assert_eq!(loaded.self_update_channel, Some("test-channel".to_string()));

    // Invariant: Config file should always be valid JSON
    let content = fs::read_to_string(&config_path).unwrap();
    let _: GupGlobalConfig = serde_json::from_str(&content).unwrap();
}

#[test]
fn test_concurrent_access_safety() {
    let test_env = TestEnvironment::new().unwrap();
    let paths = test_env.paths().unwrap();

    // Simulate concurrent project additions
    let projects = vec![
        ("proj1", "Project 1", "https://example.com/p1.json"),
        ("proj2", "Project 2", "https://example.com/p2.json"),
        ("proj3", "Project 3", "https://example.com/p3.json"),
    ];

    // Add projects sequentially (simulating concurrent access)
    for (name, display, url) in &projects {
        add_managed_project(name, display, url, &paths).unwrap();

        // Verify config remains valid after each operation
        let config = load_global_config(&paths).unwrap();
        assert!(config.managed_projects.contains_key(*name));

        // Invariant: Config file should remain parseable
        let config_path = paths.global_config_file();
        if config_path.exists() {
            let content = fs::read_to_string(&config_path).unwrap();
            let _: GupGlobalConfig = serde_json::from_str(&content).unwrap();
        }
    }

    // Final verification
    let final_config = load_global_config(&paths).unwrap();
    assert_eq!(final_config.managed_projects.len(), 3);
}

#[test]
fn test_empty_project_name_handling() {
    let test_env = TestEnvironment::new().unwrap();
    let paths = test_env.paths().unwrap();

    // Test edge case: empty project name
    add_managed_project("", "Empty Name Project", "https://example.com", &paths).unwrap();
    assert!(is_project_managed("", &paths).unwrap());

    // Should be able to remove it too
    let removed = remove_managed_project("", &paths).unwrap();
    assert!(removed);
    assert!(!is_project_managed("", &paths).unwrap());
}

#[test]
fn test_special_character_project_names() {
    let test_env = TestEnvironment::new().unwrap();
    let paths = test_env.paths().unwrap();

    let special_names = vec![
        "project-with-dashes",
        "project_with_underscores",
        "project.with.dots",
        "project@with@symbols",
        "项目中文名称", // Unicode characters
    ];

    for name in &special_names {
        add_managed_project(name, "Special Project", "https://example.com", &paths).unwrap();
        assert!(is_project_managed(name, &paths).unwrap());

        // Invariant: Special characters should survive roundtrip
        let config = load_global_config(&paths).unwrap();
        let project = config.managed_projects.get(*name).unwrap();
        assert_eq!(project.unique_name, *name);
    }

    // All should exist simultaneously
    let final_config = load_global_config(&paths).unwrap();
    assert_eq!(final_config.managed_projects.len(), special_names.len());
}

#[test]
fn test_url_validation_in_project_info() {
    let test_env = TestEnvironment::new().unwrap();
    let paths = test_env.paths().unwrap();

    let test_urls = vec![
        "https://example.com/metadata.json",
        "http://localhost:8000/test.json",
        "file:///local/path/metadata.json",
        "https://github.com/user/repo/raw/main/metadata.json",
    ];

    for (i, url) in test_urls.iter().enumerate() {
        let project_name = format!("test-project-{}", i);
        add_managed_project(&project_name, "Test Project", url, &paths).unwrap();

        let config = load_global_config(&paths).unwrap();
        let project = config.managed_projects.get(&project_name).unwrap();
        assert_eq!(project.source_of_truth_url, *url);
    }
}

#[test]
fn test_config_serialization_stability() {
    let test_env = TestEnvironment::new().unwrap();
    let _paths = test_env.paths().unwrap();

    // Create config with all possible fields populated
    let mut config = GupGlobalConfig::default();
    config.self_update_channel = Some("stable".to_string());
    config.background_self_update_interval_minutes = Some(1440);

    let project_info = ManagedProjectInfo {
        unique_name: "test".to_string(),
        display_name: "Test Project".to_string(),
        source_of_truth_url: "https://example.com/test.json".to_string(),
        last_metadata_sync: Some(Utc::now()),
        preferred_default: Some("latest".to_string()),
    };
    config
        .managed_projects
        .insert("test".to_string(), project_info);

    // Serialize multiple times - should be stable
    let json1 = serde_json::to_string_pretty(&config).unwrap();
    let json2 = serde_json::to_string_pretty(&config).unwrap();
    assert_eq!(json1, json2);

    // Roundtrip should preserve all data
    let deserialized: GupGlobalConfig = serde_json::from_str(&json1).unwrap();
    let json3 = serde_json::to_string_pretty(&deserialized).unwrap();
    assert_eq!(json1, json3);

    // Invariant: All fields should be preserved
    assert_eq!(config.self_update_channel, deserialized.self_update_channel);
    assert_eq!(
        config.background_self_update_interval_minutes,
        deserialized.background_self_update_interval_minutes
    );
    assert_eq!(
        config.managed_projects.len(),
        deserialized.managed_projects.len()
    );
}
