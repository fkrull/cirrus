use cirrus_core::{
    model::{backup, repo, Config},
    restic::{Options, Restic},
    secrets::Secrets,
};
use clap::ArgMatches;

#[cfg(feature = "desktop-integration")]
pub mod desktop;
pub mod secret;

pub async fn restic(
    restic: &Restic,
    secrets: &Secrets,
    config: &Config,
    matches: &ArgMatches<'_>,
) -> eyre::Result<()> {
    let cmd = matches.values_of("cmd").unwrap();
    match matches.value_of("repo") {
        Some(repo_name) => {
            let repo_name = repo::Name(repo_name.to_owned());
            let repo = config.repository(&repo_name)?;
            let repo_with_secrets = secrets.get_secrets(repo)?;
            restic
                .run(Some(repo_with_secrets), cmd, &Options::default())?
                .wait()
                .await
        }
        None => restic.run(None, cmd, &Options::default())?.wait().await,
    }
}

pub async fn backup(
    restic: &Restic,
    secrets: &Secrets,
    config: &Config,
    matches: &ArgMatches<'_>,
) -> eyre::Result<()> {
    let backup_name = backup::Name(matches.value_of("backup").unwrap().to_owned());
    let backup = config.backup(&backup_name)?;
    let repo = config.repository_for_backup(backup)?;
    let repo_with_secrets = secrets.get_secrets(repo)?;
    restic
        .backup(repo_with_secrets, backup, &Options::default())?
        .wait()
        .await
}
