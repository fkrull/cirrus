use crate::{parse_args, parse_env, Workdir};
use cirrus_core::{
    model::repo,
    restic::{Options, Restic},
    secrets,
};
use maplit::hashmap;

#[tokio::test]
async fn should_run_specified_restic_binary_with_explicit_arguments() {
    let workdir = Workdir::new().unwrap();
    let restic = Restic::new(Some(workdir.bin().to_owned()));

    restic
        .run(None, &["arg1", "arg2", "arg3", "arg4"], &Options::default())
        .unwrap()
        .wait()
        .await
        .unwrap();

    parse_args(workdir.path())
        .await
        .unwrap()
        .assert_args(&["arg1", "arg2", "arg3", "arg4"]);
}

#[tokio::test]
async fn should_run_restic_with_repo_parameter_and_secrets() {
    let workdir = Workdir::new().unwrap();
    let restic = Restic::new(Some(workdir.bin().to_owned()));
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

    parse_args(workdir.path()).await.unwrap().assert_args(&[
        "--repo",
        "local:/srv/repo",
        "snapshots",
    ]);
    parse_env(workdir.path())
        .await
        .unwrap()
        .assert_var("RESTIC_PASSWORD", "repo-password")
        .assert_var("SECRET1", "secret1")
        .assert_var("SECRET2", "secret2");
}
