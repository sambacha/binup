use assert_cmd::Command;

#[test]
fn command_default() {
    let depot_dir = assert_fs::TempDir::new().unwrap();

    // Test default command with no projects (should fail gracefully)
    Command::cargo_bin("gup")
        .unwrap()
        .arg("default")
        .arg("non-existent-project")
        .arg("1.0.0")
        .env("GUP_DEPOT_PATH", depot_dir.path())
        .assert()
        .failure(); // Should fail because no projects are registered

    // Test default command without arguments (should show current default or error)
    Command::cargo_bin("gup")
        .unwrap()
        .arg("default")
        .env("GUP_DEPOT_PATH", depot_dir.path())
        .assert()
        .failure(); // Should fail/show error because no defaults set
}
