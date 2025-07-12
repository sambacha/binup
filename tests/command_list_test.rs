use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn command_list() {
    let depot_dir = tempfile::Builder::new()
        .prefix("guptest")
        .tempdir()
        .unwrap();

    // Test list command when no projects are managed
    Command::cargo_bin("gup")
        .unwrap()
        .arg("list")
        .env("GUP_DEPOT_PATH", depot_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "No projects are currently managed",
        ));

    // Test that ls is an alias for list
    Command::cargo_bin("gup")
        .unwrap()
        .arg("ls")
        .env("GUP_DEPOT_PATH", depot_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "No projects are currently managed",
        ));
}
