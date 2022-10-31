use crate::{cli, Cache};
use cirrus_core::{
    config::{repo, Config},
    restic::Restic,
    secrets::Secrets,
};

pub async fn fill(
    restic: &Restic,
    secrets: &Secrets,
    config: &Config,
    cache: &Cache,
    args: cli::index::Fill,
) -> eyre::Result<()> {
    let repo_name = repo::Name(args.repo);
    let repo = config
        .repositories
        .get(&repo_name)
        .ok_or_else(|| eyre::eyre!("unknown repository {}", repo_name.0))?;
    let repo_with_secrets = secrets.get_secrets(repo)?;
    let cache_dir = cache.get().await?;
    let mut db = cirrus_index::Database::new(cache_dir, &repo_name).await?;
    let snapshots = cirrus_index::index_snapshots(restic, &mut db, &repo_with_secrets).await?;
    println!("{snapshots} snapshots saved");
    let unindexed = db.get_unindexed_snapshots(100).await?;
    println!("indexing {} unindexed snapshots...", unindexed.len());
    for snapshot in &unindexed {
        println!("indexing {}...", snapshot.short_id());
        cirrus_index::index_files(restic, &mut db, &repo_with_secrets, snapshot).await?;
    }

    Ok(())
}
