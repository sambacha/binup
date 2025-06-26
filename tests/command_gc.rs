use assert_cmd::Command;

#[test]
fn command_gc() {
    let depot_dir = assert_fs::TempDir::new().unwrap();

    // Test gc command with non-existent project (should fail gracefully)
    Command::cargo_bin("gup")
        .unwrap()
        .arg("gc")
        .arg("non-existent-project")
        .env("GUP_DEPOT_PATH", depot_dir.path())
        .assert()
        .failure(); // Should fail for non-existent project

    // Test gc with prune-linked flag and non-existent project
    Command::cargo_bin("gup")
        .unwrap()
        .arg("gc")
        .arg("non-existent-project")
        .arg("--prune-linked")
        .env("GUP_DEPOT_PATH", depot_dir.path())
        .assert()
        .failure(); // Should fail for non-existent project

    // Test status after gc operations
    Command::cargo_bin("gup")
        .unwrap()
        .arg("status")
        .env("GUP_DEPOT_PATH", depot_dir.path())
        .assert()
        .success();
}
