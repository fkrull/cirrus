use crate::new_workdir;
use assert_cmd::Command;

#[test]
fn should_run_restic_with_given_arguments() {
    let workdir = new_workdir()
        .with_stdout(b"stdout1\nstdout2\n")
        .with_file("cirrus.toml", "");
    Command::cargo_bin("test-cirrus")
        .unwrap()
        .arg("--restic-binary")
        .arg(workdir.test_binary())
        .arg("--config-file")
        .arg(workdir.path().join("cirrus.toml"))
        .args(&["restic", "snapshots"])
        .assert()
        .success()
        .stdout(b"stdout1\nstdout2\n".as_ref());
    workdir.assert_args(&["snapshots"]);
}

#[test]
fn should_run_backup() {
    let workdir = new_workdir().with_file(
        "cirrus.toml",
        toml::to_string(&toml::toml! {
            [repositories.test]
            url = "local:/srv/repo"

            [repositories.test.password]
            env_var = "TEST_PASSWORD"

            [backups.test]
            repository = "test"
            path = "/"
            exclude-caches = true
        })
        .unwrap(),
    );
    Command::cargo_bin("test-cirrus")
        .unwrap()
        .arg("--restic-binary")
        .arg(workdir.test_binary())
        .arg("--config-file")
        .arg(workdir.path().join("cirrus.toml"))
        .args(&["backup", "test"])
        .env("TEST_PASSWORD", "pwd")
        .assert()
        .success();
    workdir
        .assert_args(&[
            "--repo",
            "local:/srv/repo",
            "backup",
            "/",
            "--tag",
            "cirrus.test",
            "--exclude-caches",
        ])
        .assert_env_var("RESTIC_PASSWORD", "pwd");
}

#[test]
fn should_run_restic_subcommand_without_config_file_if_possible() {
    let workdir = new_workdir();
    Command::cargo_bin("test-cirrus")
        .unwrap()
        .arg("--restic-binary")
        .arg(workdir.test_binary())
        .arg("--config-file")
        .arg(workdir.path().join("does-not-exist.toml"))
        .args(&["restic", "version"])
        .assert()
        .success();
}
