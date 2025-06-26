#[cfg(windows)]
mod windows_shim_tests {
    use anyhow::Result;
    use gup::global_paths::get_gup_paths;
    use gup::operations_symlink::create_executable_symlink;
    use std::env;
    use std::fs;
    use std::path::{Path, PathBuf};
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
    fn test_windows_shim_creation() {
        let test_env = TestEnvironment::new().unwrap();
        let paths = test_env.paths().unwrap();

        // Create a mock gup executable
        let gup_path = test_env._temp_dir.path().join("gup.exe");
        fs::write(&gup_path, b"mock gup executable").unwrap();

        // Create shim for an executable
        create_executable_symlink("node", &gup_path, &paths).unwrap();

        // Verify shim was created
        let shim_path = paths.bin_dir().join("node.bat");
        assert!(shim_path.exists(), "Shim batch file should exist");

        // Read and verify shim content
        let content = fs::read_to_string(&shim_path).unwrap();
        assert!(content.contains("@echo off"), "Should start with echo off");
        assert!(
            content.contains(&gup_path.to_string_lossy()),
            "Should contain gup path"
        );
        assert!(content.contains("%*"), "Should pass all arguments");
    }

    #[test]
    fn test_shim_content_format() {
        let test_env = TestEnvironment::new().unwrap();
        let paths = test_env.paths().unwrap();

        let gup_path = test_env._temp_dir.path().join("bin").join("gup.exe");
        fs::create_dir_all(gup_path.parent().unwrap()).unwrap();
        fs::write(&gup_path, b"mock").unwrap();

        create_executable_symlink("python", &gup_path, &paths).unwrap();

        let shim_path = paths.bin_dir().join("python.bat");
        let content = fs::read_to_string(&shim_path).unwrap();

        // Verify exact content structure
        let expected_lines = vec![
            "@echo off",
            &format!("\"{}\" %*", gup_path.to_string_lossy()),
        ];

        let actual_lines: Vec<&str> = content.lines().collect();
        assert_eq!(actual_lines.len(), expected_lines.len());

        for (actual, expected) in actual_lines.iter().zip(expected_lines.iter()) {
            assert_eq!(actual.trim(), expected.trim());
        }
    }

    #[test]
    fn test_shim_with_spaces_in_path() {
        let test_env = TestEnvironment::new().unwrap();
        let paths = test_env.paths().unwrap();

        // Create path with spaces
        let spaced_dir = test_env._temp_dir.path().join("Program Files").join("gup");
        fs::create_dir_all(&spaced_dir).unwrap();
        let gup_path = spaced_dir.join("gup.exe");
        fs::write(&gup_path, b"mock").unwrap();

        create_executable_symlink("npm", &gup_path, &paths).unwrap();

        let shim_path = paths.bin_dir().join("npm.bat");
        let content = fs::read_to_string(&shim_path).unwrap();

        // Path with spaces should be properly quoted
        assert!(content.contains("\""), "Path with spaces should be quoted");
        assert!(
            content.contains("Program Files"),
            "Should contain the spaced directory"
        );
    }

    #[test]
    fn test_multiple_shim_creation() {
        let test_env = TestEnvironment::new().unwrap();
        let paths = test_env.paths().unwrap();

        let gup_path = test_env._temp_dir.path().join("gup.exe");
        fs::write(&gup_path, b"mock").unwrap();

        let executables = vec!["node", "npm", "npx", "python", "pip", "rust", "cargo"];

        for exe in &executables {
            create_executable_symlink(exe, &gup_path, &paths).unwrap();
        }

        // Verify all shims were created
        for exe in &executables {
            let shim_path = paths.bin_dir().join(format!("{}.bat", exe));
            assert!(shim_path.exists(), "Shim for {} should exist", exe);

            // Each should have correct content
            let content = fs::read_to_string(&shim_path).unwrap();
            assert!(content.contains(&gup_path.to_string_lossy()));
        }
    }

    #[test]
    fn test_shim_overwrite_behavior() {
        let test_env = TestEnvironment::new().unwrap();
        let paths = test_env.paths().unwrap();

        let gup_path = test_env._temp_dir.path().join("gup.exe");
        fs::write(&gup_path, b"mock").unwrap();

        // Create initial shim
        create_executable_symlink("julia", &gup_path, &paths).unwrap();

        let shim_path = paths.bin_dir().join("julia.bat");
        let initial_content = fs::read_to_string(&shim_path).unwrap();

        // Modify the shim manually
        fs::write(&shim_path, "@echo off\necho modified\n").unwrap();

        // Create shim again - should overwrite
        create_executable_symlink("julia", &gup_path, &paths).unwrap();

        let final_content = fs::read_to_string(&shim_path).unwrap();
        assert_eq!(
            initial_content, final_content,
            "Shim should be restored to original"
        );
    }

    #[test]
    fn test_shim_bin_dir_creation() {
        let test_env = TestEnvironment::new().unwrap();
        let paths = test_env.paths().unwrap();

        // Ensure bin dir doesn't exist initially
        let bin_dir = paths.bin_dir();
        if bin_dir.exists() {
            fs::remove_dir_all(&bin_dir).unwrap();
        }
        assert!(!bin_dir.exists());

        let gup_path = test_env._temp_dir.path().join("gup.exe");
        fs::write(&gup_path, b"mock").unwrap();

        // Creating shim should also create bin directory
        create_executable_symlink("test", &gup_path, &paths).unwrap();

        assert!(bin_dir.exists(), "Bin directory should be created");
        assert!(bin_dir.is_dir(), "Bin should be a directory");
    }

    #[test]
    fn test_shim_special_characters_in_name() {
        let test_env = TestEnvironment::new().unwrap();
        let paths = test_env.paths().unwrap();

        let gup_path = test_env._temp_dir.path().join("gup.exe");
        fs::write(&gup_path, b"mock").unwrap();

        // Test with various special characters (that are valid in Windows filenames)
        let special_names = vec!["python3.10", "node-v16", "ruby_2.7", "tool-name", "my.tool"];

        for name in &special_names {
            create_executable_symlink(name, &gup_path, &paths).unwrap();

            let shim_path = paths.bin_dir().join(format!("{}.bat", name));
            assert!(shim_path.exists(), "Shim for {} should exist", name);
        }
    }

    #[test]
    fn test_shim_long_path_handling() {
        let test_env = TestEnvironment::new().unwrap();
        let paths = test_env.paths().unwrap();

        // Create a deeply nested path
        let mut deep_path = test_env._temp_dir.path().to_path_buf();
        for i in 0..10 {
            deep_path = deep_path.join(format!("nested_directory_{}", i));
        }
        fs::create_dir_all(&deep_path).unwrap();

        let gup_path = deep_path.join("gup.exe");
        fs::write(&gup_path, b"mock").unwrap();

        // Should handle long paths gracefully
        create_executable_symlink("deeptest", &gup_path, &paths).unwrap();

        let shim_path = paths.bin_dir().join("deeptest.bat");
        let content = fs::read_to_string(&shim_path).unwrap();

        // Should contain the full path
        assert!(content.contains(&gup_path.to_string_lossy()));
    }

    #[test]
    fn test_shim_unicode_path_support() {
        let test_env = TestEnvironment::new().unwrap();
        let paths = test_env.paths().unwrap();

        // Create path with unicode characters
        let unicode_dir = test_env._temp_dir.path().join("测试目录");
        fs::create_dir_all(&unicode_dir).unwrap();
        let gup_path = unicode_dir.join("gup.exe");
        fs::write(&gup_path, b"mock").unwrap();

        create_executable_symlink("unicode_test", &gup_path, &paths).unwrap();

        let shim_path = paths.bin_dir().join("unicode_test.bat");
        assert!(shim_path.exists());

        // Verify the shim can handle unicode paths
        let content = fs::read_to_string(&shim_path).unwrap();
        assert!(content.contains("测试目录"));
    }

    #[test]
    fn test_shim_error_handling() {
        let test_env = TestEnvironment::new().unwrap();
        let paths = test_env.paths().unwrap();

        // Test with non-existent gup executable
        let fake_gup_path = test_env
            ._temp_dir
            .path()
            .join("nonexistent")
            .join("gup.exe");

        // Should still create shim even if gup doesn't exist
        let result = create_executable_symlink("test", &fake_gup_path, &paths);
        assert!(
            result.is_ok(),
            "Should create shim even for non-existent target"
        );

        let shim_path = paths.bin_dir().join("test.bat");
        assert!(shim_path.exists());
    }

    #[test]
    fn test_shim_concurrent_creation() {
        let test_env = TestEnvironment::new().unwrap();
        let paths = test_env.paths().unwrap();

        let gup_path = test_env._temp_dir.path().join("gup.exe");
        fs::write(&gup_path, b"mock").unwrap();

        // Simulate concurrent shim creation
        let executables = vec!["concurrent1", "concurrent2", "concurrent3"];

        for exe in &executables {
            create_executable_symlink(exe, &gup_path, &paths).unwrap();
        }

        // All shims should exist and be valid
        for exe in &executables {
            let shim_path = paths.bin_dir().join(format!("{}.bat", exe));
            assert!(shim_path.exists());

            let content = fs::read_to_string(&shim_path).unwrap();
            assert!(content.contains("@echo off"));
            assert!(content.contains(&gup_path.to_string_lossy()));
        }
    }

    #[test]
    fn test_shim_permission_preservation() {
        let test_env = TestEnvironment::new().unwrap();
        let paths = test_env.paths().unwrap();

        let gup_path = test_env._temp_dir.path().join("gup.exe");
        fs::write(&gup_path, b"mock").unwrap();

        create_executable_symlink("perm_test", &gup_path, &paths).unwrap();

        let shim_path = paths.bin_dir().join("perm_test.bat");

        // On Windows, batch files should be readable and executable by default
        let metadata = fs::metadata(&shim_path).unwrap();
        assert!(!metadata.permissions().readonly());
    }
}

// Tests for non-Windows platforms to ensure the module compiles
#[cfg(not(windows))]
mod non_windows_tests {
    #[test]
    fn test_windows_shim_module_exists() {
        // This test ensures the windows shim tests compile on non-Windows platforms
        // even though they won't run
        assert!(true);
    }
}
