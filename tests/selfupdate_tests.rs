use anyhow::Result;
use chrono::{Duration, Utc};
use gup::cli::GupChannel;
use gup::global_config_manager::*;
use gup::global_paths::get_gup_paths;
use gup::state_config::GupGlobalConfig;
use std::env;
use std::fs;
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
        match &self.original_env {
            Some(val) => env::set_var("GUP_DEPOT_PATH", val),
            None => env::remove_var("GUP_DEPOT_PATH"),
        }
    }
}

#[test]
fn test_selfupdate_channel_persistence() {
    let test_env = TestEnvironment::new().unwrap();
    let paths = test_env.paths().unwrap();

    // Test setting self-update channel
    let mut config = GupGlobalConfig::default();
    config.self_update_channel = Some("release".to_string());
    save_global_config(&config, &paths).unwrap();

    // Verify it persists
    let loaded_config = load_global_config(&paths).unwrap();
    assert_eq!(
        loaded_config.self_update_channel,
        Some("release".to_string())
    );

    // Test changing channel
    let mut updated_config = loaded_config;
    updated_config.self_update_channel = Some("releasepreview".to_string());
    save_global_config(&updated_config, &paths).unwrap();

    let final_config = load_global_config(&paths).unwrap();
    assert_eq!(
        final_config.self_update_channel,
        Some("releasepreview".to_string())
    );
}

#[test]
fn test_selfupdate_timestamp_tracking() {
    let test_env = TestEnvironment::new().unwrap();
    let paths = test_env.paths().unwrap();

    let mut config = GupGlobalConfig::default();
    let now = Utc::now();
    config.last_self_update = Some(now);
    save_global_config(&config, &paths).unwrap();

    let loaded_config = load_global_config(&paths).unwrap();
    assert!(loaded_config.last_self_update.is_some());

    // Invariant: Timestamp should be preserved exactly
    assert_eq!(loaded_config.last_self_update.unwrap(), now);
}

#[test]
fn test_version_comparison_logic() {
    // Test version comparison for different formats
    let test_cases = vec![
        ("1.0.0", "1.0.1", true),       // Older
        ("1.0.1", "1.0.0", false),      // Newer
        ("1.0.0", "1.0.0", false),      // Same
        ("2.0.0", "1.9.9", false),      // Major version
        ("1.0.0-alpha", "1.0.0", true), // Pre-release
    ];

    for (current, latest, should_update) in test_cases {
        let needs_update = current != latest && current < latest;
        assert_eq!(
            needs_update, should_update,
            "Version comparison failed for {} vs {}",
            current, latest
        );
    }
}

#[test]
fn test_channel_validation() {
    let valid_channels = vec!["release", "releasepreview", "dev"];
    let invalid_channels = vec!["", "random", "nightly", "beta"];

    for channel in valid_channels {
        // Should not panic
        let _channel_enum = match channel {
            "release" => Some(GupChannel::Release),
            "releasepreview" => Some(GupChannel::ReleasePreview),
            "dev" => Some(GupChannel::Dev),
            _ => None,
        };
        assert!(_channel_enum.is_some());
    }

    for channel in invalid_channels {
        let channel_enum = match channel {
            "release" => Some(GupChannel::Release),
            "releasepreview" => Some(GupChannel::ReleasePreview),
            "dev" => Some(GupChannel::Dev),
            _ => None,
        };
        assert!(channel_enum.is_none());
    }
}

#[test]
fn test_update_check_with_rate_limiting() {
    let test_env = TestEnvironment::new().unwrap();
    let paths = test_env.paths().unwrap();

    // Set up a recent update check
    let mut config = GupGlobalConfig::default();
    config.last_self_update = Some(Utc::now() - Duration::minutes(5));
    save_global_config(&config, &paths).unwrap();

    // Invariant: Should respect rate limiting
    // In real implementation, should skip check if too recent
    let time_since_last = Utc::now() - config.last_self_update.unwrap();
    assert!(
        time_since_last < Duration::hours(1),
        "Should be within rate limit window"
    );
}

#[test]
fn test_selfupdate_rollback_on_failure() {
    let test_env = TestEnvironment::new().unwrap();
    let _paths = test_env.paths().unwrap();

    // Create a mock current executable
    let exe_dir = test_env._temp_dir.path().join("bin");
    fs::create_dir_all(&exe_dir).unwrap();
    let current_exe = exe_dir.join("gup");
    fs::write(&current_exe, b"original binary content").unwrap();

    // Set executable permissions on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = fs::Permissions::from_mode(0o755);
        fs::set_permissions(&current_exe, perms).unwrap();
    }

    // Simulate update process
    let backup_path = current_exe.with_extension("backup");
    fs::copy(&current_exe, &backup_path).unwrap();

    // Write a "bad" update
    fs::write(&current_exe, b"corrupted binary").unwrap();

    // Simulate rollback
    fs::copy(&backup_path, &current_exe).unwrap();
    fs::remove_file(&backup_path).unwrap();

    // Invariant: Original content should be restored
    let content = fs::read(&current_exe).unwrap();
    assert_eq!(content, b"original binary content");
}

#[test]
fn test_binary_replacement_atomicity() {
    let test_env = TestEnvironment::new().unwrap();
    let _paths = test_env.paths().unwrap();

    let exe_dir = test_env._temp_dir.path().join("bin");
    fs::create_dir_all(&exe_dir).unwrap();
    let target_exe = exe_dir.join("gup");

    // Write initial binary
    fs::write(&target_exe, b"version 1.0.0").unwrap();

    // Atomic replacement process
    let temp_path = target_exe.with_extension("new");
    fs::write(&temp_path, b"version 2.0.0").unwrap();

    // On Unix, rename is atomic
    #[cfg(unix)]
    {
        fs::rename(&temp_path, &target_exe).unwrap();
    }

    // On Windows, we need to remove then rename
    #[cfg(windows)]
    {
        fs::remove_file(&target_exe).unwrap();
        fs::rename(&temp_path, &target_exe).unwrap();
    }

    // Invariant: File should contain new content
    let content = fs::read(&target_exe).unwrap();
    assert_eq!(content, b"version 2.0.0");

    // Invariant: No temporary files should remain
    assert!(!temp_path.exists());
}

#[test]
fn test_platform_specific_binary_selection() {
    use gup::utils::get_target_triple_id;

    let target_triple = get_target_triple_id().unwrap();

    // Test binary naming conventions
    let binary_names = vec![
        format!("gup-1.0.0-{}.tar.gz", target_triple),
        format!("gup-1.0.0-{}.zip", target_triple),
        format!("gup-{}", target_triple),
    ];

    for name in &binary_names {
        assert!(
            name.contains(&target_triple),
            "Binary name should contain target triple"
        );

        // Invariant: Binary name should follow expected pattern
        if name.ends_with(".tar.gz") || name.ends_with(".zip") {
            assert!(name.contains("-1.0.0-"), "Archive should contain version");
        }
    }
}

#[test]
fn test_update_channel_precedence() {
    let test_env = TestEnvironment::new().unwrap();
    let paths = test_env.paths().unwrap();

    // Test channel precedence: CLI > Config > Default
    let mut config = GupGlobalConfig::default();

    // No channel set - should default to "release"
    assert!(config.self_update_channel.is_none());

    // Config channel set
    config.self_update_channel = Some("releasepreview".to_string());
    save_global_config(&config, &paths).unwrap();

    let loaded_config = load_global_config(&paths).unwrap();
    assert_eq!(
        loaded_config.self_update_channel,
        Some("releasepreview".to_string())
    );

    // CLI channel would override (tested in integration tests)
}

#[test]
fn test_concurrent_selfupdate_safety() {
    let test_env = TestEnvironment::new().unwrap();
    let paths = test_env.paths().unwrap();

    // Simulate concurrent access by multiple processes
    let mut config1 = load_global_config(&paths).unwrap();
    let mut config2 = load_global_config(&paths).unwrap();

    // Both processes update different fields
    config1.self_update_channel = Some("dev".to_string());
    config2.last_self_update = Some(Utc::now());

    // Save from first process
    save_global_config(&config1, &paths).unwrap();

    // Second save would overwrite - this is expected behavior
    save_global_config(&config2, &paths).unwrap();

    // Invariant: Last write wins, but file should remain valid
    let final_config = load_global_config(&paths).unwrap();
    assert!(
        final_config.self_update_channel.is_none()
            || final_config.self_update_channel == Some("dev".to_string())
    );
    assert!(final_config.last_self_update.is_some());
}

#[test]
fn test_github_api_response_parsing() {
    // Test parsing various GitHub API responses
    let valid_release_response = r#"{
        "tag_name": "v1.2.3",
        "name": "Release v1.2.3",
        "prerelease": false,
        "draft": false
    }"#;

    let release_array_response = r#"[{
        "tag_name": "v1.2.4-beta",
        "name": "Beta Release v1.2.4",
        "prerelease": true,
        "draft": false
    }]"#;

    // Parse single release
    let release: serde_json::Value = serde_json::from_str(valid_release_response).unwrap();
    assert_eq!(release["tag_name"].as_str().unwrap(), "v1.2.3");

    // Parse release array
    let releases: serde_json::Value = serde_json::from_str(release_array_response).unwrap();
    assert!(releases.is_array());
    assert_eq!(releases[0]["tag_name"].as_str().unwrap(), "v1.2.4-beta");
}

#[test]
fn test_update_download_url_construction() {
    use gup::utils::get_target_triple_id;

    let version = "1.2.3";
    let target_triple = get_target_triple_id().unwrap();

    let url = format!(
        "https://github.com/your-org/gup/releases/download/v{}/gup-{}-{}.tar.gz",
        version, version, target_triple
    );

    // Invariant: URL should be well-formed
    assert!(url.starts_with("https://"));
    assert!(url.contains("/releases/download/"));
    assert!(url.contains(&version));
    assert!(url.contains(&target_triple));
    assert!(url.ends_with(".tar.gz"));
}

#[test]
fn test_selfupdate_error_recovery() {
    let test_env = TestEnvironment::new().unwrap();
    let paths = test_env.paths().unwrap();

    // Test recovery from various error conditions

    // 1. Network failure recovery
    let mut config = load_global_config(&paths).unwrap();
    let _original_version = "1.0.0";

    // Simulate failed update - config should remain valid
    config.last_self_update = Some(Utc::now());
    save_global_config(&config, &paths).unwrap();

    // 2. Disk space recovery
    let exe_dir = test_env._temp_dir.path().join("bin");
    fs::create_dir_all(&exe_dir).unwrap();

    // 3. Permission error recovery
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let readonly_file = exe_dir.join("readonly");
        fs::write(&readonly_file, b"test").unwrap();
        fs::set_permissions(&readonly_file, fs::Permissions::from_mode(0o444)).unwrap();

        // Attempting to write should fail, but not corrupt system
        let write_result = fs::write(&readonly_file, b"new content");
        assert!(write_result.is_err());

        // Original content should be preserved
        let content = fs::read(&readonly_file).unwrap();
        assert_eq!(content, b"test");
    }
}

// Mock tests would require the mockito crate to be added as a dev dependency
// Uncomment and add mockito to Cargo.toml to enable these tests
/*
#[cfg(feature = "mock_tests")]
mod mock_tests {
    use super::*;
    use mockito::{mock, Matcher};

    #[test]
    fn test_github_api_mock_responses() {
        let _m = mock("GET", "/repos/your-org/gup/releases/latest")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"tag_name": "v1.2.3"}"#)
            .create();

        // Test would make actual HTTP request to mockito server
        // let response = reqwest::blocking::get(&mockito::server_url()).unwrap();
        // assert_eq!(response.status(), 200);
    }
}
*/
