use crate::model::{backup, repo};
use crate::restic::Options;
use crate::Cirrus;
use clap::ArgMatches;

pub mod secret;

pub async fn restic(app: &Cirrus, matches: &ArgMatches<'_>) -> anyhow::Result<()> {
    let cmd = matches.values_of("cmd").unwrap();
    match matches.value_of("repo") {
        Some(repo_name) => {
            let repo_name = repo::Name(repo_name.to_owned());
            let repo = app.config.repository(&repo_name)?;
            let repo_with_secrets = app.secrets.get_secrets(repo)?;
            app.restic
                .run(Some(repo_with_secrets), cmd, &Options::default())?
                .wait()
                .await
        }
        None => app.restic.run(None, cmd, &Options::default())?.wait().await,
    }
}

pub async fn backup(app: &Cirrus, matches: &ArgMatches<'_>) -> anyhow::Result<()> {
    let backup_name = backup::Name(matches.value_of("backup").unwrap().to_owned());
    let backup = app.config.backup(&backup_name)?;
    let repo = app.config.repository_for_backup(backup)?;
    let repo_with_secrets = app.secrets.get_secrets(repo)?;
    app.restic
        .backup(repo_with_secrets, backup, &Options::default())?
        .wait()
        .await
}
