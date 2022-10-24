use rusqlite_migration::{Migrations, M};
use tokio::task::block_in_place;

fn migrations() -> Migrations<'static> {
    Migrations::new(vec![
        //language=SQLite
        M::up(
            r#"
CREATE TABLE snapshots(
    repo TEXT NOT NULL,
    id TEXT NOT NULL,
    short_id TEXT NOT NULL,
    parent TEXT,
    tree TEXT NOT NULL,
    hostname TEXT NOT NULL,
    username TEXT NOT NULL,
    time TEXT NOT NULL,
    tags TEXT NOT NULL,
    PRIMARY KEY (repo, id)
);"#,
        ),
    ])
}

pub(crate) async fn apply_migrations(conn: &mut rusqlite::Connection) -> eyre::Result<()> {
    block_in_place(|| migrations().to_latest(conn))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migrations() {
        assert!(migrations().validate().is_ok());
    }
}
