use crate::cli;
use cirrus_core::{
    config::{repo, Config},
    restic::Restic,
    secrets::Secrets,
};
use dirs_next as dirs;
use std::path::PathBuf;

const CACHEDIR_TAG_FILENAME: &str = "CACHEDIR.TAG";

const CACHEDIR_TAG_CONTENT: &str = "Signature: 8a477f597d28d172789f06886806bc55
# This file is a cache directory tag created by cirrus.
# For information about cache directory tags see https://bford.info/cachedir/
";

async fn cache_dir() -> eyre::Result<PathBuf> {
    let path = dirs::cache_dir()
        .ok_or_else(|| eyre::eyre!("can't determine cache directory"))?
        .join("cirrus");
    tokio::fs::create_dir_all(&path).await?;
    tokio::fs::write(path.join(CACHEDIR_TAG_FILENAME), CACHEDIR_TAG_CONTENT).await?;
    Ok(path)
}

pub async fn fill(
    restic: &Restic,
    secrets: &Secrets,
    config: &Config,
    args: cli::index::Fill,
) -> eyre::Result<()> {
    let repo_name = repo::Name(args.repo);
    let repo = config
        .repositories
        .get(&repo_name)
        .ok_or_else(|| eyre::eyre!("unknown repository {}", repo_name.0))?;
    let repo_with_secrets = secrets.get_secrets(repo)?;
    let cache_dir = cache_dir().await?;
    let mut db = cirrus_index::Database::new(&cache_dir, &repo_name).await?;
    let snapshots = cirrus_index::index_snapshots(restic, &mut db, &repo_with_secrets).await?;
    println!("{snapshots} snapshots indexed");

    let unindexed = db.get_unindexed_snapshots(100).await?;
    println!("indexing {} unindexed snapshots...", unindexed.len());
    for snapshot in &unindexed {
        println!("indexing {}...", snapshot.short_id());
        cirrus_index::index_files(restic, &mut db, &repo_with_secrets, snapshot).await?;
    }

    Ok(())
}
