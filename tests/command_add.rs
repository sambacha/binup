use assert_cmd::Command;
use predicates::prelude::predicate;

#[test]
fn command_add() {
    let depot_dir = assert_fs::TempDir::new().unwrap();

    // Test adding a non-existent project - should fail gracefully
    Command::cargo_bin("gup")
        .unwrap()
        .arg("add")
        .arg("non-existent-project")
        .arg("1.0.0")
        .env("GUP_DEPOT_PATH", depot_dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("Project not found"));

    // Test adding with invalid arguments - should show usage
    Command::cargo_bin("gup")
        .unwrap()
        .arg("add")
        .env("GUP_DEPOT_PATH", depot_dir.path())
        .assert()
        .failure();

    // Test adding project without version - should fail
    Command::cargo_bin("gup")
        .unwrap()
        .arg("add")
        .arg("some-project")
        .env("GUP_DEPOT_PATH", depot_dir.path())
        .assert()
        .failure();
}
