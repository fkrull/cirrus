use crate::{
    cli::repo_contents::{Cli, Cmd, Index, Ls},
    Cache,
};
use cirrus_core::{
    config::{repo, Config},
    restic::Restic,
    secrets::Secrets,
};
use cirrus_index::{File, FileSize, Parent, Type};
use term_grid::{Cell, Grid, GridOptions};
use time::{format_description::FormatItem, macros::format_description, OffsetDateTime};

pub async fn repo_contents(
    restic: &Restic,
    secrets: &Secrets,
    config: &Config,
    cache: &Cache,
    args: Cli,
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
    args: Index,
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

async fn ls(cache: &Cache, repo_name: &repo::Name, args: Ls) -> eyre::Result<()> {
    const MAX_ITEMS: u64 = 10000;
    let cache_dir = cache.get().await?;
    let mut db = cirrus_index::Database::new(cache_dir, repo_name).await?;
    let path = parse_path(&args.path);
    let entries = db.get_files(&path, MAX_ITEMS + 1).await?;

    let count = entries.len();
    if count == 1 {
        println!("{count} item");
    } else if count as u64 <= MAX_ITEMS {
        println!("{count} items");
    } else {
        println!("More than {MAX_ITEMS} items (truncated)");
    }

    let mut grid = Grid::new(GridOptions {
        direction: term_grid::Direction::LeftToRight,
        filling: term_grid::Filling::Spaces(3),
    });
    let now = OffsetDateTime::now_local()?;
    for (file, version, snapshot) in entries.into_iter().take(1000) {
        grid.add(Cell::from(format_name(&file)));
        grid.add(Cell::from(
            version.size.map(format_size).unwrap_or_default(),
        ));
        grid.add(Cell::from(format_time(snapshot.time, now)));
        grid.add(Cell::from(format!("on {}", snapshot.hostname)));
        grid.add(Cell::from(snapshot.snapshot_id.short_id().to_string()));
    }
    println!("{}", grid.fit_into_columns(5));
    Ok(())
}

// TODO: test
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

fn format_name(file: &File) -> String {
    if file.r#type == Type::Dir {
        format!("{}/", file.name)
    } else {
        file.name.clone()
    }
}

fn format_size(bytes: FileSize) -> String {
    humansize::format_size(bytes.0, humansize::BINARY)
}

fn format_time(time: OffsetDateTime, local_now: OffsetDateTime) -> String {
    let local_time = time.to_offset(local_now.offset());
    const TODAY_FORMAT: &'static [FormatItem<'static>] = format_description!("[hour]:[minute]");
    const YESTERDAY_FORMAT: &'static [FormatItem<'static>] =
        format_description!("yesterday [hour]:[minute]");
    const LAST_WEEK_FORMAT: &'static [FormatItem<'static>] =
        format_description!("[weekday repr:short] [hour]:[minute]");
    const THIS_YEAR_FORMAT: &'static [FormatItem<'static>] =
        format_description!("[day] [month repr:short]");
    const FALLBACK_FORMAT: &'static [FormatItem<'static>] =
        format_description!("[day] [month repr:short] [year]");
    let day_diff = local_now.to_julian_day() - local_time.to_julian_day();
    let format = if day_diff == 0 {
        TODAY_FORMAT
    } else if day_diff == 1 {
        YESTERDAY_FORMAT
    } else if day_diff < 7 {
        LAST_WEEK_FORMAT
    } else if local_time.year() == local_now.year() {
        THIS_YEAR_FORMAT
    } else {
        FALLBACK_FORMAT
    };
    local_time.format(format).expect("formattable time")
}
