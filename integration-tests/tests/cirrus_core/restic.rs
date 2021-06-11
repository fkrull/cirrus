use crate::new_workdir;
use cirrus_core::restic::Verbosity;
use cirrus_core::{
    model::{backup, repo},
    restic::{Options, Restic},
    secrets,
};
use maplit::hashmap;
use std::collections::HashMap;

#[tokio::test]
async fn should_run_specified_restic_binary_with_explicit_arguments() {
    let workdir = new_workdir();
    let restic = Restic::new(Some(workdir.test_binary().to_owned()));

    restic
        .run(None, &["arg1", "arg2", "arg3", "arg4"], &Options::default())
        .unwrap()
        .wait()
        .await
        .unwrap();

    workdir
        .args()
        .unwrap()
        .assert_args(&["arg1", "arg2", "arg3", "arg4"]);
}

#[tokio::test]
async fn should_run_restic_with_repo_parameter_and_secrets() {
    let workdir = new_workdir();
    let restic = Restic::new(Some(workdir.test_binary().to_owned()));
    let repo = repo::Definition {
        url: repo::Url("local:/srv/repo".to_owned()),
        password: repo::Secret::FromEnvVar {
            env_var: "REPO_PWD".to_owned(),
        },
        secrets: hashmap! {
            repo::SecretName("SECRET1".to_owned()) =>
                repo::Secret::FromEnvVar { env_var: "SECRET1_SOURCE".to_owned() },
            repo::SecretName("SECRET2".to_owned()) =>
                repo::Secret::FromEnvVar { env_var: "SECRET2_SOURCE".to_owned() },
        },
    };
    let repo_with_secrets = secrets::RepoWithSecrets {
        repo: &repo,
        repo_password: secrets::SecretValue("repo-password".to_owned()),
        secrets: hashmap! {
            repo::SecretName("SECRET1".to_owned()) => secrets::SecretValue("secret1".to_owned()),
            repo::SecretName("SECRET2".to_owned()) => secrets::SecretValue("secret2".to_owned()),
        },
    };

    restic
        .run(Some(repo_with_secrets), &["snapshots"], &Options::default())
        .unwrap()
        .wait()
        .await
        .unwrap();

    workdir
        .args()
        .unwrap()
        .assert_args(&["--repo", "local:/srv/repo", "snapshots"]);
    workdir
        .env()
        .unwrap()
        .assert_var("RESTIC_PASSWORD", "repo-password")
        .assert_var("SECRET1", "secret1")
        .assert_var("SECRET2", "secret2");
}

#[cfg(windows)]
const EXCLUDE_PARAM: &'static str = "--iexclude";
#[cfg(not(windows))]
const EXCLUDE_PARAM: &'static str = "--exclude";

#[tokio::test]
async fn should_run_restic_backup() {
    let workdir = new_workdir();
    let restic = Restic::new(Some(workdir.test_binary().to_owned()));
    let repo = repo::Definition {
        url: repo::Url("local:/srv/repo".to_owned()),
        password: repo::Secret::FromEnvVar {
            env_var: "".to_owned(),
        },
        secrets: HashMap::new(),
    };
    let repo_with_secrets = secrets::RepoWithSecrets {
        repo: &repo,
        repo_password: secrets::SecretValue("repo-password".to_owned()),
        secrets: HashMap::new(),
    };
    let backup_name = backup::Name("bkp".to_owned());
    let backup = backup::Definition {
        repository: repo::Name("repo".to_owned()),
        path: backup::Path("/home/test".to_owned()),
        excludes: vec![backup::Exclude(".Trash".to_owned())],
        exclude_caches: true,
        extra_args: vec!["--one-file-system".to_owned()],
        disable_triggers: false,
        triggers: vec![],
    };

    restic
        .backup(
            repo_with_secrets,
            &backup_name,
            &backup,
            &Options::default(),
        )
        .unwrap()
        .wait()
        .await
        .unwrap();

    workdir.args().unwrap().assert_args(&[
        "--repo",
        "local:/srv/repo",
        "backup",
        "/home/test",
        "--tag",
        "cirrus-backup-bkp",
        EXCLUDE_PARAM,
        ".Trash",
        "--exclude-caches",
        "--one-file-system",
    ]);
}

#[tokio::test]
async fn should_run_restic_with_options() {
    let workdir = new_workdir();
    let restic = Restic::new(Some(workdir.test_binary().to_owned()));

    restic
        .run(
            None,
            std::iter::empty::<&str>(),
            &Options {
                json: true,
                verbose: Verbosity::VVV,
                ..Default::default()
            },
        )
        .unwrap()
        .wait()
        .await
        .unwrap();

    workdir
        .args()
        .unwrap()
        .assert_args(&["--json", "--verbose=3"]);
}
