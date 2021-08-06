use cirrus_core::{
    model::{backup, repo, Config},
    restic::{Options, Restic},
    secrets::Secrets,
};

use crate::cli;

pub mod daemon;
#[cfg(feature = "desktop-commands")]
pub mod desktop;
pub mod generate;
pub mod secret;

pub async fn restic(
    restic: &Restic,
    secrets: &Secrets,
    config: &Config,
    args: cli::restic::Cli,
) -> eyre::Result<()> {
    match args.repository {
        Some(repo_name) => {
            let repo_name = repo::Name(repo_name.to_owned());
            let repo = config.repository(&repo_name)?;
            let repo_with_secrets = secrets.get_secrets(repo)?;
            restic
                .run(Some(&repo_with_secrets), &args.cmd, &Options::default())?
                .check_wait()
                .await?
        }
        None => {
            restic
                .run(None, &args.cmd, &Options::default())?
                .check_wait()
                .await?
        }
    }
    Ok(())
}

pub async fn backup(
    restic: &Restic,
    secrets: &Secrets,
    config: &Config,
    args: cli::backup::Cli,
) -> eyre::Result<()> {
    let backup_name = backup::Name(args.backup);
    let backup = config.backup(&backup_name)?;
    let repo = config.repository_for_backup(backup)?;
    let repo_with_secrets = secrets.get_secrets(repo)?;
    restic
        .backup(
            &repo_with_secrets,
            &backup_name,
            backup,
            &Options::default(),
        )?
        .check_wait()
        .await?;
    Ok(())
}

pub fn config(config: &Config) -> eyre::Result<()> {
    print!("{}", toml::to_string_pretty(config)?);
    Ok(())
}

pub async fn version(restic: &Restic) -> eyre::Result<()> {
    println!(
        "cirrus: {}",
        cirrus_core::VERSION.unwrap_or("[untagged build]")
    );
    match restic.version_string().await {
        Ok(restic_version) => println!("restic: {}", restic_version),
        Err(err) => println!(
            "Could not determine restic version ({}), is restic installed correctly?",
            err
        ),
    }
    Ok(())
}
