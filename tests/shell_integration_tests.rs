use anyhow::Result;
use gup::global_paths::get_gup_paths;
use std::env;
use std::fs;
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
        Ok(TestEnvironment {
            _temp_dir: temp_dir,
            original_env,
            original_home,
        })
    }

    fn paths(&self) -> Result<gup::global_paths::GupGlobalPaths> {
        get_gup_paths()
    }

    fn create_test_shell_file(
        &self,
        shell_type: &str,
        content: &str,
    ) -> Result<std::path::PathBuf> {
        let filename = match shell_type {
            "bash" => ".bashrc",
            "zsh" => ".zshrc",
            "fish" => ".config/fish/config.fish",
            "profile" => ".profile",
            "bash_profile" => ".bash_profile",
            _ => panic!("Unknown shell type"),
        };

        let path = if filename.contains('/') {
            let parts: Vec<&str> = filename.split('/').collect();
            let dir = self
                ._temp_dir
                .path()
                .join(parts[0..parts.len() - 1].join("/"));
            fs::create_dir_all(&dir)?;
            self._temp_dir.path().join(filename)
        } else {
            self._temp_dir.path().join(filename)
        };

        fs::write(&path, content)?;
        Ok(path)
    }

    fn set_as_home(&self) {
        env::set_var("HOME", self._temp_dir.path());
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
fn test_find_shell_scripts_basic() {
    let test_env = TestEnvironment::new().unwrap();

    // Create some shell files
    test_env.create_test_shell_file("bash", "# bashrc").unwrap();
    test_env.create_test_shell_file("zsh", "# zshrc").unwrap();
    test_env
        .create_test_shell_file("profile", "# profile")
        .unwrap();

    test_env.set_as_home();

    // Find scripts to modify (add_case = false, only existing files)
    let scripts = gup::operations_shell::find_shell_scripts_to_be_modified(false).unwrap();
    assert_eq!(scripts.len(), 3);

    // Verify the found scripts
    let script_names: Vec<String> = scripts
        .iter()
        .filter_map(|p| p.file_name())
        .map(|name| name.to_string_lossy().to_string())
        .collect();

    assert!(script_names.contains(&".bashrc".to_string()));
    assert!(script_names.contains(&".zshrc".to_string()));
    assert!(script_names.contains(&".profile".to_string()));
}

#[test]
fn test_find_shell_scripts_macos_zshrc_creation() {
    if std::env::consts::OS != "macos" {
        return; // Skip on non-macOS
    }

    let test_env = TestEnvironment::new().unwrap();
    test_env.set_as_home();

    // On macOS with add_case = true, should find .zshrc even if it doesn't exist
    let scripts = gup::operations_shell::find_shell_scripts_to_be_modified(true).unwrap();

    let has_zshrc = scripts
        .iter()
        .any(|p| p.file_name().map(|name| name == ".zshrc").unwrap_or(false));

    assert!(
        has_zshrc,
        "Should find .zshrc on macOS even if it doesn't exist"
    );
}

#[test]
fn test_add_binfolder_to_path_basic() {
    let test_env = TestEnvironment::new().unwrap();
    let paths = test_env.paths().unwrap();
    let bin_path = paths.gup_bin_dir();

    // Create test shell files
    test_env
        .create_test_shell_file("bash", "export EDITOR=vim\n")
        .unwrap();
    test_env
        .create_test_shell_file("zsh", "# zsh config\n")
        .unwrap();

    test_env.set_as_home();

    // Add bin folder to path
    gup::operations_shell::add_binfolder_to_path_in_shell_scripts(&bin_path).unwrap();

    // Check bashrc was modified
    let bashrc_path = test_env._temp_dir.path().join(".bashrc");
    let bashrc_content = fs::read_to_string(&bashrc_path).unwrap();
    assert!(bashrc_content.contains(">>> gup initialize >>>"));
    assert!(bashrc_content.contains("<<< gup initialize <<<"));
    assert!(bashrc_content.contains(&*bin_path.to_string_lossy()));
    assert!(bashrc_content.contains("export EDITOR=vim")); // Original content preserved

    // Check zshrc was modified with zsh-specific syntax
    let zshrc_path = test_env._temp_dir.path().join(".zshrc");
    let zshrc_content = fs::read_to_string(&zshrc_path).unwrap();
    assert!(zshrc_content.contains(">>> gup initialize >>>"));
    assert!(zshrc_content.contains("<<< gup initialize <<<"));
    assert!(zshrc_content.contains(&*bin_path.to_string_lossy()));
    assert!(zshrc_content.contains("path=(")); // zsh-specific syntax
}

#[test]
fn test_add_binfolder_idempotency() {
    let test_env = TestEnvironment::new().unwrap();
    let paths = test_env.paths().unwrap();
    let bin_path = paths.gup_bin_dir();

    test_env.create_test_shell_file("bash", "# test\n").unwrap();
    test_env.set_as_home();

    // Add path first time
    gup::operations_shell::add_binfolder_to_path_in_shell_scripts(&bin_path).unwrap();

    let bashrc_path = test_env._temp_dir.path().join(".bashrc");
    let _content_after_first = fs::read_to_string(&bashrc_path).unwrap();

    // Add path second time (should be idempotent)
    gup::operations_shell::add_binfolder_to_path_in_shell_scripts(&bin_path).unwrap();

    let content_after_second = fs::read_to_string(&bashrc_path).unwrap();

    // Should not have duplicate sections
    let marker_count = content_after_second
        .matches(">>> gup initialize >>>")
        .count();
    assert_eq!(marker_count, 1, "Should have exactly one gup section");
}

#[test]
fn test_shell_script_with_complex_content() {
    let test_env = TestEnvironment::new().unwrap();
    let paths = test_env.paths().unwrap();
    let bin_path = paths.gup_bin_dir();

    // Create bashrc with complex content
    let complex_content = r#"#!/bin/bash
# Complex bashrc

# PATH modifications
export PATH="/usr/local/bin:$PATH"
export PATH="$HOME/.cargo/bin:$PATH"

# Functions
function mkcd() {
    mkdir -p "$1" && cd "$1"
}

# Aliases
alias ll='ls -la'
alias gs='git status'

# Source other files
if [ -f ~/.bash_aliases ]; then
    . ~/.bash_aliases
fi

# Node Version Manager
export NVM_DIR="$HOME/.nvm"
[ -s "$NVM_DIR/nvm.sh" ] && \. "$NVM_DIR/nvm.sh"
"#;

    test_env
        .create_test_shell_file("bash", complex_content)
        .unwrap();
    test_env.set_as_home();

    gup::operations_shell::add_binfolder_to_path_in_shell_scripts(&bin_path).unwrap();

    let bashrc_path = test_env._temp_dir.path().join(".bashrc");
    let modified_content = fs::read_to_string(&bashrc_path).unwrap();

    // Verify gup section was added
    assert!(modified_content.contains(">>> gup initialize >>>"));

    // Verify all original content is preserved
    assert!(modified_content.contains("#!/bin/bash"));
    assert!(modified_content.contains("export PATH=\"/usr/local/bin:$PATH\""));
    assert!(modified_content.contains("function mkcd()"));
    assert!(modified_content.contains("alias ll='ls -la'"));
    assert!(modified_content.contains("export NVM_DIR="));
}

#[test]
fn test_nonexistent_home_directory() {
    let _test_env = TestEnvironment::new().unwrap();

    // Set HOME to a non-existent directory
    env::set_var("HOME", "/nonexistent/path/that/does/not/exist");

    // Should return empty list
    let scripts = gup::operations_shell::find_shell_scripts_to_be_modified(false).unwrap();
    assert_eq!(scripts.len(), 0);
}

#[test]
fn test_path_with_special_characters() {
    let test_env = TestEnvironment::new().unwrap();

    // Create a bin path with spaces and special characters
    let special_bin = test_env
        ._temp_dir
        .path()
        .join("Program Files")
        .join("gup & tools")
        .join("bin");
    fs::create_dir_all(&special_bin).unwrap();

    test_env.create_test_shell_file("bash", "").unwrap();
    test_env.set_as_home();

    gup::operations_shell::add_binfolder_to_path_in_shell_scripts(&special_bin).unwrap();

    let bashrc_path = test_env._temp_dir.path().join(".bashrc");
    let content = fs::read_to_string(&bashrc_path).unwrap();

    // Should contain the path with special characters
    assert!(content.contains("Program Files"));
    assert!(content.contains("gup & tools"));
}

#[test]
fn test_empty_shell_files() {
    let test_env = TestEnvironment::new().unwrap();
    let paths = test_env.paths().unwrap();
    let bin_path = paths.gup_bin_dir();

    // Create empty shell files
    test_env.create_test_shell_file("bash", "").unwrap();
    test_env.create_test_shell_file("zsh", "").unwrap();
    test_env.create_test_shell_file("profile", "").unwrap();

    test_env.set_as_home();

    gup::operations_shell::add_binfolder_to_path_in_shell_scripts(&bin_path).unwrap();

    // All files should have content now
    for filename in &[".bashrc", ".zshrc", ".profile"] {
        let path = test_env._temp_dir.path().join(filename);
        let content = fs::read_to_string(&path).unwrap();
        assert!(!content.is_empty());
        assert!(content.contains(">>> gup initialize >>>"));
    }
}

#[test]
fn test_shell_script_permissions() {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let test_env = TestEnvironment::new().unwrap();
        let paths = test_env.paths().unwrap();
        let bin_path = paths.gup_bin_dir();

        let bashrc_path = test_env.create_test_shell_file("bash", "# test\n").unwrap();

        // Set specific permissions
        let perms = fs::Permissions::from_mode(0o644);
        fs::set_permissions(&bashrc_path, perms).unwrap();

        test_env.set_as_home();

        gup::operations_shell::add_binfolder_to_path_in_shell_scripts(&bin_path).unwrap();

        // Check permissions are preserved
        let metadata = fs::metadata(&bashrc_path).unwrap();
        let mode = metadata.permissions().mode();
        assert_eq!(mode & 0o777, 0o644);
    }
}
