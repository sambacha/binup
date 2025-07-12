use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

/// Test helper to set up a test environment
struct TestEnvironment {
    _temp_dir: TempDir,
    depot_path: std::path::PathBuf,
}

impl TestEnvironment {
    fn new() -> Self {
        let temp_dir = TempDir::new().unwrap();
        let depot_path = temp_dir.path().to_path_buf();

        Self {
            _temp_dir: temp_dir,
            depot_path,
        }
    }

    fn gup_command(&self) -> Command {
        let mut cmd = Command::cargo_bin("gup").unwrap();
        cmd.env("GUP_DEPOT_PATH", &self.depot_path);
        cmd
    }
}

#[test]
fn test_gup_basic_commands() {
    let env = TestEnvironment::new();

    // Test help command
    env.gup_command()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Generic Updater/Installer Program",
        ));

    // Test project list when empty
    env.gup_command()
        .arg("project")
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "No projects are currently managed",
        ));
}

#[test]
fn test_error_handling() {
    let env = TestEnvironment::new();

    // Try to add version for non-existent project
    env.gup_command()
        .arg("add")
        .arg("nonexistent-project")
        .arg("1.0.0")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Project not found"));

    // Try to list non-existent project
    env.gup_command()
        .arg("list")
        .arg("nonexistent-project")
        .assert()
        .failure()
        .stderr(predicate::str::contains("is not managed"));

    // Try to add project with invalid URL
    env.gup_command()
        .arg("project")
        .arg("add")
        .arg("not-a-url")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid URL"));
}

#[test]
fn test_status_command() {
    let env = TestEnvironment::new();

    // Status with no projects should work
    env.gup_command().arg("status").assert().success();
}

#[test]
fn test_invalid_commands() {
    let env = TestEnvironment::new();

    // Invalid subcommand
    env.gup_command().arg("invalid-command").assert().failure();

    // Missing arguments
    env.gup_command().arg("add").assert().failure();

    env.gup_command()
        .arg("project")
        .arg("add")
        .assert()
        .failure();
}

#[test]
fn test_completions_generation() {
    let env = TestEnvironment::new();

    // Test completion generation for different shells
    let shells = ["bash", "zsh", "fish", "powershell"];

    for shell in &shells {
        env.gup_command()
            .arg("completions")
            .arg(shell)
            .assert()
            .success()
            .stdout(predicate::str::is_empty().not());
    }
}

// Additional test for command parsing and basic functionality
#[test]
fn test_command_parsing() {
    let env = TestEnvironment::new();

    // Test various command combinations that should parse but may fail at execution

    // Test 'project' subcommand structure
    env.gup_command()
        .arg("project")
        .arg("list")
        .assert()
        .success();

    // Test 'list' command with no project (should show no projects message)
    env.gup_command()
        .arg("list")
        .assert()
        .success() // Actually succeeds but shows message about no projects
        .stdout(predicate::str::contains(
            "No projects are currently managed",
        ));
}
