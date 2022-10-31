use crate::cli::repo_contents::Cmd;
use crate::{cli, Cache};
use cirrus_core::{
    config::{repo, Config},
    restic::Restic,
    secrets::Secrets,
};

pub async fn repo_contents(
    restic: &Restic,
    secrets: &Secrets,
    config: &Config,
    cache: &Cache,
    args: cli::repo_contents::Cli,
) -> eyre::Result<()> {
    let repo_name = repo::Name(args.repository);
    let repo = config
        .repositories
        .get(&repo_name)
        .ok_or_else(|| eyre::eyre!("unknown repository {}", repo_name.0))?;
    match args.subcommand {
        Cmd::Index(args) => index(restic, secrets, cache, &repo_name, repo, args).await,
    }
}

async fn index(
    restic: &Restic,
    secrets: &Secrets,
    cache: &Cache,
    repo_name: &repo::Name,
    repo: &repo::Definition,
    args: cli::repo_contents::Index,
) -> eyre::Result<()> {
    let cache_dir = cache.get().await?;
    let repo_with_secrets = secrets.get_secrets(repo)?;
    let mut db = cirrus_index::Database::new(cache_dir, repo_name).await?;
    let snapshots = cirrus_index::index_snapshots(restic, &mut db, &repo_with_secrets).await?;
    println!("{snapshots} snapshots saved");
    let to_index = db
        .get_unindexed_snapshots(args.snapshots_count as u64)
        .await?;
    println!("indexing {} snapshots...", to_index.len());
    for snapshot in &to_index {
        println!("indexing {}...", snapshot.short_id());
        cirrus_index::index_files(restic, &mut db, &repo_with_secrets, snapshot).await?;
    }
    Ok(())
}
