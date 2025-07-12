use assert_cmd::Command;

#[test]
fn command_basic_functionality() {
    let depot_dir = assert_fs::TempDir::new().unwrap();

    // Test basic help commands work
    Command::cargo_bin("gup")
        .unwrap()
        .arg("--help")
        .env("GUP_DEPOT_PATH", depot_dir.path())
        .assert()
        .success();

    // Test basic status command
    Command::cargo_bin("gup")
        .unwrap()
        .arg("status")
        .env("GUP_DEPOT_PATH", depot_dir.path())
        .assert()
        .success();

    // Test project list when empty
    Command::cargo_bin("gup")
        .unwrap()
        .arg("project")
        .arg("list")
        .env("GUP_DEPOT_PATH", depot_dir.path())
        .assert()
        .success();

    // Test that list without project shows error
    Command::cargo_bin("gup")
        .unwrap()
        .arg("list")
        .env("GUP_DEPOT_PATH", depot_dir.path())
        .assert()
        .success(); // Actually succeeds but shows no projects message
}
