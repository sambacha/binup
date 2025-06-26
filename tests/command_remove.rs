use assert_cmd::Command;
use predicates::boolean::PredicateBooleanExt;

#[test]
#[ignore] // This test needs to be updated for GUP's project-based architecture
fn command_remove() {
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
        .stdout(predicates::str::contains("1.6.4").not());

    Command::cargo_bin("gup")
        .unwrap()
        .arg("add")
        .arg("1.6.4")
        .env("GUP_DEPOT_PATH", depot_dir.path())
        .env("GUP_DEPOT_PATH", depot_dir.path())
        .assert()
        .success()
        .stdout("");

    Command::cargo_bin("gup")
        .unwrap()
        .arg("status")
        .env("GUP_DEPOT_PATH", depot_dir.path())
        .env("GUP_DEPOT_PATH", depot_dir.path())
        .assert()
        .success()
        .stdout(predicates::str::contains("1.6.4"));

    Command::cargo_bin("gup")
        .unwrap()
        .arg("add")
        .arg("release")
        .env("GUP_DEPOT_PATH", depot_dir.path())
        .env("GUP_DEPOT_PATH", depot_dir.path())
        .assert()
        .success()
        .stdout("");

    Command::cargo_bin("gup")
        .unwrap()
        .arg("status")
        .env("GUP_DEPOT_PATH", depot_dir.path())
        .env("GUP_DEPOT_PATH", depot_dir.path())
        .assert()
        .success()
        .stdout(predicates::str::contains("1.6.4").and(predicates::str::contains("release")));

    Command::cargo_bin("gup")
        .unwrap()
        .arg("remove")
        .arg("release")
        .env("GUP_DEPOT_PATH", depot_dir.path())
        .env("GUP_DEPOT_PATH", depot_dir.path())
        .assert()
        .success()
        .stdout("");

    Command::cargo_bin("gup")
        .unwrap()
        .arg("status")
        .env("GUP_DEPOT_PATH", depot_dir.path())
        .env("GUP_DEPOT_PATH", depot_dir.path())
        .assert()
        .success()
        .stdout(predicates::str::contains("1.6.4").and(predicates::str::contains("release").not()));

    Command::cargo_bin("gup")
        .unwrap()
        .arg("add")
        .arg("nightly")
        .env("GUP_DEPOT_PATH", depot_dir.path())
        .env("GUP_DEPOT_PATH", depot_dir.path())
        .assert()
        .success()
        .stdout("");

    Command::cargo_bin("gup")
        .unwrap()
        .arg("status")
        .env("GUP_DEPOT_PATH", depot_dir.path())
        .env("GUP_DEPOT_PATH", depot_dir.path())
        .assert()
        .success()
        .stdout(predicates::str::contains("1.6.4").and(predicates::str::contains("-DEV")));

    Command::cargo_bin("gup")
        .unwrap()
        .arg("remove")
        .arg("nightly")
        .env("GUP_DEPOT_PATH", depot_dir.path())
        .env("GUP_DEPOT_PATH", depot_dir.path())
        .assert()
        .success()
        .stdout("");

    Command::cargo_bin("gup")
        .unwrap()
        .arg("status")
        .env("GUP_DEPOT_PATH", depot_dir.path())
        .env("GUP_DEPOT_PATH", depot_dir.path())
        .assert()
        .success()
        .stdout(predicates::str::contains("1.6.4").and(predicates::str::contains("-DEV").not()));
}
