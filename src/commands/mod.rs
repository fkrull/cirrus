use crate::model::{backup, repo};
use crate::Cirrus;
use clap::ArgMatches;

pub mod secret;

pub fn restic(app: &Cirrus, matches: &ArgMatches) -> anyhow::Result<()> {
    let cmd = matches.values_of("cmd").unwrap();
    match matches.value_of("repo") {
        Some(repo_name) => {
            let repo_name = repo::Name(repo_name.to_owned());
            let repo = app.config.repository(&repo_name)?;
            let secrets = app.secrets.get_secrets(repo)?;
            app.restic.run(repo, &secrets, cmd)?.wait_blocking()?;
        }
        None => {
            app.restic.run_raw(cmd)?.wait_blocking()?;
        }
    }

    Ok(())
}

pub fn backup(app: &Cirrus, matches: &ArgMatches) -> anyhow::Result<()> {
    let backup_name = backup::Name(matches.value_of("backup").unwrap().to_owned());
    let backup = app.config.backup(&backup_name)?;
    let repo = app.config.repository_for_backup(backup)?;
    let secrets = app.secrets.get_secrets(repo)?;
    app.restic.backup(repo, &secrets, backup)?.wait_blocking()
}
