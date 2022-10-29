use crate::cli;
use cirrus_core::{
    config::{repo, Config},
    restic::Restic,
    secrets::Secrets,
};
use dirs_next as dirs;
use std::path::PathBuf;

fn index_file() -> eyre::Result<PathBuf> {
    let path = dirs::cache_dir()
        .ok_or_else(|| eyre::eyre!("can't determine cache directory"))?
        .join("cirrus")
        .join("index.sqlite");
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
    let path = index_file()?;
    // TODO cleaner error handling and everything
    tokio::fs::create_dir_all(path.parent().unwrap()).await?;
    let mut db = cirrus_index::Database::new(path).await?;
    let snapshots = cirrus_index::index_snapshots(restic, &mut db, &repo_with_secrets).await?;
    println!("{snapshots} snapshots indexed");

    /*let unindexed = db.get_unindexed_snapshots(&repo, 10).await?;
    println!("indexing {} unindexed snapshots...", unindexed.len());
    for snapshot in &unindexed {
        println!("indexing {}...", snapshot.short_id);
        cirrus_index::index_files(restic, &mut db, &repo_with_secrets, &snapshot.key).await?;
    }*/

    Ok(())
}
