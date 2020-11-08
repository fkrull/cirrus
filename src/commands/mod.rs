use cirrus_core::{
    model::{backup, repo, Config},
    restic::{Options, Restic},
    secrets::Secrets,
};
use clap::ArgMatches;

#[cfg(feature = "desktop-commands")]
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

pub fn list_repos(config: &Config, _matches: &ArgMatches<'_>) -> eyre::Result<()> {
    let repos: Vec<_> = config
        .repositories
        .iter()
        .map(|(name, definition)| (name.0.as_str(), definition.url.0.as_str()))
        .collect();
    print_table(&repos);
    Ok(())
}

pub fn list_backups(config: &Config, _matches: &ArgMatches<'_>) -> eyre::Result<()> {
    let backups: Vec<_> = config
        .backups
        .iter()
        .map(|(name, definition)| (name.0.as_str(), definition.path.0.as_str()))
        .collect();
    print_table(&backups);
    Ok(())
}

fn print_table(rows: &[(&str, &str)]) -> Option<()> {
    let a_max = rows.iter().map(|r| r.0.len()).max()?;
    let b_max = rows.iter().map(|r| r.1.len()).max()?;
    for &(a, b) in rows {
        print!("{}    {}", padded(a, a_max), padded(b, b_max));
    }
    Some(())
}

fn padded(s: &str, to: usize) -> String {
    let mut buf = String::with_capacity(to);
    buf.push_str(s);
    for _ in s.len()..to {
        buf.push(' ');
    }
    buf
}
