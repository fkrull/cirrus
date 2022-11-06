use crate::cli::repo_contents::Cmd;
use crate::{cli, Cache};
use cirrus_core::{
    config::{repo, Config},
    restic::Restic,
    secrets::Secrets,
};
use cirrus_index::Parent;

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
        Cmd::Ls(args) => ls(cache, &repo_name, args).await,
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

async fn ls(
    cache: &Cache,
    repo_name: &repo::Name,
    args: cli::repo_contents::Ls,
) -> eyre::Result<()> {
    let cache_dir = cache.get().await?;
    let mut db = cirrus_index::Database::new(cache_dir, repo_name).await?;
    let path = parse_path(&args.path);
    println!("{}", path.to_path());
    let entries = db.get_files(&path, 1001).await?;
    let count = entries.len();
    for (file, version, snapshot) in entries.into_iter().take(1000) {
        println!(
            "  {} [{:?}] {} {:?} {}",
            file.name,
            file.r#type,
            version
                .size
                .map(|o| o.0.to_string())
                .unwrap_or_else(|| "-".to_string()),
            snapshot.time,
            snapshot.hostname
        );
    }
    if count > 1000 {
        println!("...truncated");
    }
    Ok(())
}

// TODO: nicer, test, index crate?
fn parse_path(s: &str) -> Parent {
    let trimmed = s.trim().trim_end_matches("/").trim();
    if trimmed.is_empty() {
        Parent(None)
    } else if !trimmed.starts_with("/") {
        Parent(Some(format!("/{trimmed}")))
    } else {
        Parent(Some(trimmed.to_string()))
    }
}
