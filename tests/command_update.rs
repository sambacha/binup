use assert_cmd::Command;

#[test]
fn command_update() {
    let depot_dir = tempfile::Builder::new()
        .prefix("guptest")
        .tempdir()
        .unwrap();

    Command::cargo_bin("gup")
        .unwrap()
        .arg("update")
        .env("GUP_DEPOT_PATH", depot_dir.path())
        .env("GUP_DEPOT_PATH", depot_dir.path())
        .assert()
        .success()
        .stdout("");

    Command::cargo_bin("gup")
        .unwrap()
        .arg("up")
        .env("GUP_DEPOT_PATH", depot_dir.path())
        .env("GUP_DEPOT_PATH", depot_dir.path())
        .assert()
        .success()
        .stdout("");
}
