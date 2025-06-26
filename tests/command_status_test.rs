use assert_cmd::Command;

#[test]
fn command_status() {
    let depot_dir = tempfile::Builder::new()
        .prefix("guptest")
        .tempdir()
        .unwrap();

    Command::cargo_bin("gup")
        .unwrap()
        .arg("status")
        .env("GUP_DEPOT_PATH", depot_dir.path())
        .env("GUP_DEPOT_PATH", depot_dir.path())
        .assert()
        .success()
        .stdout("No projects are currently managed by gup.\nUse `gup project add <registration_file_or_url>` to add one.\n");

    Command::cargo_bin("gup")
        .unwrap()
        .arg("st")
        .env("GUP_DEPOT_PATH", depot_dir.path())
        .env("GUP_DEPOT_PATH", depot_dir.path())
        .assert()
        .success()
        .stdout("No projects are currently managed by gup.\nUse `gup project add <registration_file_or_url>` to add one.\n");
}
