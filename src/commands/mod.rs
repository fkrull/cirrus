use crate::cli;
use cirrus_core::{
    config::{backup, repo, Config},
    restic::{Options, Restic},
    secrets::Secrets,
};

pub mod daemon;
pub mod repo_contents;
pub mod secret;

pub async fn restic(
    restic: &Restic,
    secrets: &Secrets,
    maybe_config: eyre::Result<Config>,
    args: cli::restic::Cli,
) -> eyre::Result<()> {
    match args.repository {
        Some(repo_name) => {
            let repo_name = repo::Name(repo_name.to_owned());
            let config = maybe_config?;
            let repo = config.repository(&repo_name)?;
            let repo_with_secrets = secrets.get_secrets(repo)?;
            restic
                .run(
                    Some(&repo_with_secrets),
                    &args.cmd,
                    &Options::inherit_output(),
                )?
                .check_wait()
                .await?
        }
        None => {
            restic
                .run(None, &args.cmd, &Options::inherit_output())?
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
            &Options::inherit_output(),
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
    if let Some(version) = cirrus_core::VERSION {
        println!("cirrus: {}", version);
    } else {
        println!("cirrus: [untagged build]")
    }

    match restic.version_string().await {
        Ok(restic_version) => println!("restic: {}", restic_version),
        Err(err) => println!(
            "Could not determine restic version ({}), is restic installed correctly?",
            err
        ),
    }
    Ok(())
}
