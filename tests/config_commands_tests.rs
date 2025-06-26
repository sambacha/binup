use anyhow::Result;
#[cfg(feature = "selfupdate")]
use gup::command_config_backgroundselfupdate::run_command_config_backgroundselfupdate;
use gup::command_config_modifypath::run_command_config_modifypath;
#[cfg(feature = "selfupdate")]
use gup::command_config_startupselfupdate::run_command_config_startupselfupdate;
use gup::global_config_manager::*;
use gup::global_paths::get_gup_paths;
use gup::state_config::GupGlobalConfig;
use std::env;
use tempfile::TempDir;

/// Test helper for creating isolated test environments
struct TestEnvironment {
    _temp_dir: TempDir,
    original_env: Option<String>,
    original_home: Option<String>,
}

impl TestEnvironment {
    fn new() -> Result<Self> {
        let temp_dir = TempDir::new()?;
        let original_env = env::var("GUP_DEPOT_PATH").ok();
        let original_home = env::var("HOME").ok();
        env::set_var("GUP_DEPOT_PATH", temp_dir.path());
        // Set HOME to temp dir to avoid modifying real shell files
        env::set_var("HOME", temp_dir.path());
        Ok(TestEnvironment {
            _temp_dir: temp_dir,
            original_env,
            original_home,
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
        match &self.original_home {
            Some(val) => env::set_var("HOME", val),
            None => env::remove_var("HOME"),
        }
    }
}

#[test]
#[cfg(feature = "selfupdate")]
fn test_backgroundselfupdate_command() {
    let test_env = TestEnvironment::new().unwrap();
    let paths = test_env.paths().unwrap();

    // Test setting background self-update interval
    run_command_config_backgroundselfupdate(Some(60), false, &paths).unwrap();

    let config = load_global_config(&paths).unwrap();
    assert_eq!(config.background_self_update_interval_minutes, Some(60));

    // Test disabling (setting to 0)
    run_command_config_backgroundselfupdate(Some(0), false, &paths).unwrap();

    let config = load_global_config(&paths).unwrap();
    assert_eq!(config.background_self_update_interval_minutes, None);

    // Test clearing (None)
    run_command_config_backgroundselfupdate(None, false, &paths).unwrap();

    let config = load_global_config(&paths).unwrap();
    assert_eq!(config.background_self_update_interval_minutes, None);
}

#[test]
#[cfg(feature = "selfupdate")]
fn test_backgroundselfupdate_quiet_mode() {
    let test_env = TestEnvironment::new().unwrap();
    let paths = test_env.paths().unwrap();

    // Test quiet mode doesn't panic
    run_command_config_backgroundselfupdate(Some(120), true, &paths).unwrap();

    let config = load_global_config(&paths).unwrap();
    assert_eq!(config.background_self_update_interval_minutes, Some(120));
}

#[test]
#[cfg(feature = "selfupdate")]
fn test_backgroundselfupdate_value_persistence() {
    let test_env = TestEnvironment::new().unwrap();
    let paths = test_env.paths().unwrap();

    // Set multiple values and ensure they persist correctly
    let test_values = vec![30, 60, 120, 1440, 10080]; // 30min, 1hr, 2hr, 1day, 1week

    for value in test_values {
        run_command_config_backgroundselfupdate(Some(value), false, &paths).unwrap();

        let config = load_global_config(&paths).unwrap();
        assert_eq!(
            config.background_self_update_interval_minutes,
            Some(value as u64)
        );

        // Ensure other config values are preserved
        assert_eq!(config.self_update_channel, None);
        assert_eq!(config.startup_self_update_interval_minutes, None);
    }
}

#[test]
#[cfg(feature = "selfupdate")]
fn test_startupselfupdate_command() {
    let test_env = TestEnvironment::new().unwrap();
    let paths = test_env.paths().unwrap();

    // Test setting startup self-update interval
    run_command_config_startupselfupdate(Some(30), false, &paths).unwrap();

    let config = load_global_config(&paths).unwrap();
    assert_eq!(config.startup_self_update_interval_minutes, Some(30));

    // Test disabling (setting to 0)
    run_command_config_startupselfupdate(Some(0), false, &paths).unwrap();

    let config = load_global_config(&paths).unwrap();
    assert_eq!(config.startup_self_update_interval_minutes, None);

    // Test clearing (None)
    run_command_config_startupselfupdate(None, false, &paths).unwrap();

    let config = load_global_config(&paths).unwrap();
    assert_eq!(config.startup_self_update_interval_minutes, None);
}

#[test]
#[cfg(feature = "selfupdate")]
fn test_startupselfupdate_interaction_with_other_settings() {
    let test_env = TestEnvironment::new().unwrap();
    let paths = test_env.paths().unwrap();

    // Set background interval first
    run_command_config_backgroundselfupdate(Some(120), false, &paths).unwrap();

    // Set startup interval
    run_command_config_startupselfupdate(Some(60), false, &paths).unwrap();

    // Both should be preserved
    let config = load_global_config(&paths).unwrap();
    assert_eq!(config.background_self_update_interval_minutes, Some(120));
    assert_eq!(config.startup_self_update_interval_minutes, Some(60));

    // Modify one shouldn't affect the other
    run_command_config_startupselfupdate(Some(90), false, &paths).unwrap();

    let config = load_global_config(&paths).unwrap();
    assert_eq!(config.background_self_update_interval_minutes, Some(120));
    assert_eq!(config.startup_self_update_interval_minutes, Some(90));
}

#[test]
fn test_modifypath_command() {
    let test_env = TestEnvironment::new().unwrap();
    let paths = test_env.paths().unwrap();

    // Test enabling PATH modification
    run_command_config_modifypath(Some(true), false, &paths).unwrap();

    let config = load_global_config(&paths).unwrap();
    assert_eq!(config.modify_path_on_install, Some(true));

    // Test disabling PATH modification
    run_command_config_modifypath(Some(false), false, &paths).unwrap();

    let config = load_global_config(&paths).unwrap();
    assert_eq!(config.modify_path_on_install, Some(false));

    // Test clearing (None)
    run_command_config_modifypath(None, false, &paths).unwrap();

    let config = load_global_config(&paths).unwrap();
    assert_eq!(config.modify_path_on_install, None);
}

#[test]
fn test_modifypath_state_transitions() {
    let test_env = TestEnvironment::new().unwrap();
    let paths = test_env.paths().unwrap();

    // Test all state transitions
    let transitions = vec![
        (None, Some(true)),
        (Some(true), Some(false)),
        (Some(false), Some(true)),
        (Some(true), None),
        (None, Some(false)),
        (Some(false), None),
    ];

    for (from_state, to_state) in transitions {
        // Set initial state
        let mut config = load_global_config(&paths).unwrap();
        config.modify_path_on_install = from_state;
        save_global_config(&config, &paths).unwrap();

        // Apply transition
        run_command_config_modifypath(to_state, false, &paths).unwrap();

        // Verify final state
        let final_config = load_global_config(&paths).unwrap();
        assert_eq!(final_config.modify_path_on_install, to_state);
    }
}

#[test]
#[cfg(feature = "selfupdate")]
fn test_config_commands_with_existing_projects() {
    let test_env = TestEnvironment::new().unwrap();
    let paths = test_env.paths().unwrap();

    // Add some projects first
    add_managed_project("nodejs", "Node.js", "https://example.com/node.json", &paths).unwrap();
    add_managed_project(
        "python",
        "Python",
        "https://example.com/python.json",
        &paths,
    )
    .unwrap();

    // Set various config values
    run_command_config_backgroundselfupdate(Some(60), false, &paths).unwrap();
    run_command_config_startupselfupdate(Some(30), false, &paths).unwrap();
    run_command_config_modifypath(Some(true), false, &paths).unwrap();

    // Verify all settings are preserved along with projects
    let config = load_global_config(&paths).unwrap();
    assert_eq!(config.background_self_update_interval_minutes, Some(60));
    assert_eq!(config.startup_self_update_interval_minutes, Some(30));
    assert_eq!(config.modify_path_on_install, Some(true));
    assert_eq!(config.managed_projects.len(), 2);
    assert!(config.managed_projects.contains_key("nodejs"));
    assert!(config.managed_projects.contains_key("python"));
}

#[test]
#[cfg(feature = "selfupdate")]
fn test_config_commands_error_recovery() {
    let test_env = TestEnvironment::new().unwrap();
    let paths = test_env.paths().unwrap();

    // Set initial valid config
    run_command_config_backgroundselfupdate(Some(60), false, &paths).unwrap();
    run_command_config_startupselfupdate(Some(30), false, &paths).unwrap();
    run_command_config_modifypath(Some(true), false, &paths).unwrap();

    // Corrupt the config file
    let config_path = paths.global_config_file();
    std::fs::write(&config_path, "{ invalid json }").unwrap();

    // Commands should handle corrupted config gracefully
    // They create a new default config if parsing fails
    let result = run_command_config_backgroundselfupdate(Some(120), false, &paths);
    assert!(result.is_ok(), "Should handle corrupted config");

    // New config should have the new value
    let config = load_global_config(&paths).unwrap();
    assert_eq!(config.background_self_update_interval_minutes, Some(120));
}

#[test]
#[cfg(feature = "selfupdate")]
fn test_config_value_boundaries() {
    let test_env = TestEnvironment::new().unwrap();
    let paths = test_env.paths().unwrap();

    // Test extreme values
    let extreme_values = vec![
        (0, None),                         // Minimum (disabled)
        (1, Some(1)),                      // 1 minute
        (60, Some(60)),                    // 1 hour
        (1440, Some(1440)),                // 1 day
        (10080, Some(10080)),              // 1 week
        (43200, Some(43200)),              // 30 days
        (525600, Some(525600)),            // 1 year
        (i64::MAX, Some(i64::MAX as u64)), // Maximum value
    ];

    for (value, expected) in extreme_values {
        // Background self-update
        run_command_config_backgroundselfupdate(Some(value), false, &paths).unwrap();
        let config = load_global_config(&paths).unwrap();
        assert_eq!(config.background_self_update_interval_minutes, expected);

        // Startup self-update
        run_command_config_startupselfupdate(Some(value), false, &paths).unwrap();
        let config = load_global_config(&paths).unwrap();
        assert_eq!(config.startup_self_update_interval_minutes, expected);
    }
}

#[test]
#[cfg(feature = "selfupdate")]
fn test_config_commands_atomicity() {
    let test_env = TestEnvironment::new().unwrap();
    let paths = test_env.paths().unwrap();

    // Set initial config
    run_command_config_backgroundselfupdate(Some(60), false, &paths).unwrap();
    run_command_config_startupselfupdate(Some(30), false, &paths).unwrap();
    run_command_config_modifypath(Some(true), false, &paths).unwrap();

    let _initial_config = load_global_config(&paths).unwrap();

    // Perform multiple updates rapidly
    for i in 0..10 {
        run_command_config_backgroundselfupdate(Some(60 + i), false, &paths).unwrap();
        run_command_config_startupselfupdate(Some(30 + i), false, &paths).unwrap();
        run_command_config_modifypath(Some(i % 2 == 0), false, &paths).unwrap();

        // Config should always be valid after each operation
        let config = load_global_config(&paths).unwrap();
        assert!(config.background_self_update_interval_minutes.is_some());
        assert!(config.startup_self_update_interval_minutes.is_some());
        assert!(config.modify_path_on_install.is_some());
    }

    // Final state should reflect last updates
    let final_config = load_global_config(&paths).unwrap();
    assert_eq!(
        final_config.background_self_update_interval_minutes,
        Some(69)
    );
    assert_eq!(final_config.startup_self_update_interval_minutes, Some(39));
    assert_eq!(final_config.modify_path_on_install, Some(false));
}

#[test]
#[cfg(feature = "selfupdate")]
fn test_config_commands_preserve_unknown_fields() {
    let test_env = TestEnvironment::new().unwrap();
    let paths = test_env.paths().unwrap();

    // Create config with all known fields
    let mut config = GupGlobalConfig::default();
    config.background_self_update_interval_minutes = Some(60);
    config.startup_self_update_interval_minutes = Some(30);
    config.modify_path_on_install = Some(true);
    config.self_update_channel = Some("stable".to_string());
    save_global_config(&config, &paths).unwrap();

    // Modify individual fields and ensure others are preserved
    run_command_config_backgroundselfupdate(Some(120), false, &paths).unwrap();

    let config = load_global_config(&paths).unwrap();
    assert_eq!(config.background_self_update_interval_minutes, Some(120));
    assert_eq!(config.startup_self_update_interval_minutes, Some(30));
    assert_eq!(config.modify_path_on_install, Some(true));
    assert_eq!(config.self_update_channel, Some("stable".to_string()));

    run_command_config_startupselfupdate(Some(60), false, &paths).unwrap();

    let config = load_global_config(&paths).unwrap();
    assert_eq!(config.background_self_update_interval_minutes, Some(120));
    assert_eq!(config.startup_self_update_interval_minutes, Some(60));
    assert_eq!(config.modify_path_on_install, Some(true));
    assert_eq!(config.self_update_channel, Some("stable".to_string()));

    run_command_config_modifypath(Some(false), false, &paths).unwrap();

    let config = load_global_config(&paths).unwrap();
    assert_eq!(config.background_self_update_interval_minutes, Some(120));
    assert_eq!(config.startup_self_update_interval_minutes, Some(60));
    assert_eq!(config.modify_path_on_install, Some(false));
    assert_eq!(config.self_update_channel, Some("stable".to_string()));
}

#[test]
#[cfg(feature = "selfupdate")]
fn test_quiet_mode_consistency() {
    let test_env = TestEnvironment::new().unwrap();
    let paths = test_env.paths().unwrap();

    // Test that quiet mode produces same results as verbose mode
    let test_cases = vec![
        (Some(60), Some(30), Some(true)),
        (Some(120), Some(60), Some(false)),
        (None, None, None),
    ];

    for (bg_val, startup_val, path_val) in test_cases {
        // Run in quiet mode
        run_command_config_backgroundselfupdate(bg_val, true, &paths).unwrap();
        run_command_config_startupselfupdate(startup_val, true, &paths).unwrap();
        run_command_config_modifypath(path_val, true, &paths).unwrap();

        let quiet_config = load_global_config(&paths).unwrap();

        // Reset and run in verbose mode
        let config = GupGlobalConfig::default();
        save_global_config(&config, &paths).unwrap();

        run_command_config_backgroundselfupdate(bg_val, false, &paths).unwrap();
        run_command_config_startupselfupdate(startup_val, false, &paths).unwrap();
        run_command_config_modifypath(path_val, false, &paths).unwrap();

        let verbose_config = load_global_config(&paths).unwrap();

        // Results should be identical
        assert_eq!(
            quiet_config.background_self_update_interval_minutes,
            verbose_config.background_self_update_interval_minutes
        );
        assert_eq!(
            quiet_config.startup_self_update_interval_minutes,
            verbose_config.startup_self_update_interval_minutes
        );
        assert_eq!(
            quiet_config.modify_path_on_install,
            verbose_config.modify_path_on_install
        );
    }
}
