use crate::{Node, Snapshot, SnapshotKey};
use cirrus_core::config::repo;
use futures::{Stream, StreamExt};
use rusqlite::{params, Connection, OptionalExtension};
use rusqlite_migration::{Migrations, M};
use std::{borrow::Borrow, path::PathBuf};

async fn b<T>(f: impl FnOnce() -> T) -> T {
    tokio::task::block_in_place(f)
}

#[derive(Debug)]
pub struct Database {
    conn: Connection,
}

impl Database {
    pub async fn new(path: impl Into<PathBuf>) -> eyre::Result<Self> {
        let path = path.into();
        let mut conn = b(|| Connection::open(&path)).await?;
        b(|| prepare_connection(&mut conn)).await?;
        b(|| migrations().to_latest(&mut conn)).await?;
        Ok(Database { conn })
    }

    pub async fn get_snapshots(&mut self, repo: &repo::Definition) -> eyre::Result<Vec<Snapshot>> {
        b(|| {
            let mut stmt = self
                .conn
                //language=SQLite
                .prepare("SELECT * FROM snapshots WHERE repo_url = ? ORDER BY time DESC")?;
            let rows = stmt.query(&[&repo.url.0])?;
            let snapshots = serde_rusqlite::from_rows(rows).collect::<Result<_, _>>()?;
            Ok(snapshots)
        })
        .await
    }

    pub async fn get_unindexed_snapshots(
        &mut self,
        repo: &repo::Definition,
        limit: u64,
    ) -> eyre::Result<Vec<Snapshot>> {
        let snapshots = b(|| {
            let mut stmt = self
                .conn
                //language=SQLite
                .prepare("SELECT * FROM snapshots WHERE repo_url = ? AND files = 0 ORDER BY time DESC LIMIT ?")?;
            let rows = stmt.query(params![&repo.url.0, limit])?;
            serde_rusqlite::from_rows(rows).collect::<Result<_, _>>()
        }).await?;
        Ok(snapshots)
    }

    pub(crate) async fn save_snapshots(
        &mut self,
        repo: &repo::Definition,
        snapshots: impl IntoIterator<Item = impl Borrow<Snapshot>>,
    ) -> eyre::Result<u64> {
        b(|| {
            let tx = self.conn.transaction()?;

            let generation = tx
                //language=SQLite
                .query_row("SELECT generation FROM snapshots LIMIT 1", [], |r| r.get(0))
                .optional()?
                .unwrap_or(0);
            let next_gen = generation + 1;
            let mut stmt = tx.prepare(
                //language=SQLite
                "
                    INSERT OR
                    REPLACE INTO snapshots(generation,
                                           repo_url,
                                           snapshot_id,
                                           backup,
                                           short_id,
                                           parent,
                                           tree,
                                           hostname,
                                           username,
                                           time,
                                           tags)
                    VALUES (:generation,
                            :repo_url,
                            :snapshot_id,
                            :backup,
                            :short_id,
                            :parent,
                            :tree,
                            :hostname,
                            :username,
                            :time,
                            :tags)",
            )?;
            let mut count = 0;
            for snapshot in snapshots {
                let snapshot = snapshot.borrow();
                let mut params = serde_rusqlite::to_params_named(snapshot)?;
                params.push((":generation".to_owned(), Box::new(next_gen)));
                stmt.execute(&*params.to_slice())?;
                count += 1;
            }
            tx.execute(
                //language=SQLite
                "DELETE FROM snapshots WHERE repo_url = ? AND generation != ? ",
                params![&repo.url.0, next_gen],
            )?;
            drop(stmt);
            tx.commit()?;
            Ok(count)
        })
        .await
    }

    pub(crate) async fn save_nodes(
        &mut self,
        snapshot: &SnapshotKey,
        nodes: impl Stream<Item = eyre::Result<Node>>,
    ) -> eyre::Result<u64> {
        let tx = b(|| self.conn.transaction()).await?;
        let generation = b(|| {
            //language=SQLite
            tx.query_row("SELECT generation FROM nodes LIMIT 1", [], |r| r.get(0))
                .optional()
        })
        .await?
        .unwrap_or(0);
        let next_gen = generation + 1;
        //language=SQLite
        let mut stmt = b(|| {
            tx.prepare(
                "
                    INSERT OR
                    REPLACE INTO nodes(generation,
                                       repo_url,
                                       snapshot_id,
                                       path,
                                       name,
                                       type,
                                       parent,
                                       uid,
                                       gid,
                                       size,
                                       mode,
                                       permissions_string,
                                       mtime,
                                       atime,
                                       ctime)
                    VALUES (:generation,
                            :repo_url,
                            :snapshot_id,
                            :path,
                            :name,
                            :type,
                            :parent,
                            :uid,
                            :gid,
                            :size,
                            :mode,
                            :permissions_string,
                            :mtime,
                            :atime,
                            :ctime)",
            )
        })
        .await?;
        let mut count = 0;
        tokio::pin!(nodes);
        while let Some(node) = nodes.next().await {
            let node = node?;
            let mut params = serde_rusqlite::to_params_named(node)?;
            params.push((":generation".to_owned(), Box::new(next_gen)));
            b(|| stmt.execute(&*params.to_slice())).await?;
            count += 1;
        }
        b(|| {
            tx.execute(
                //language=SQLite
                "DELETE FROM nodes WHERE repo_url = ? AND snapshot_id = ? AND generation != ? ",
                params![&snapshot.repo_url.0, &snapshot.snapshot_id.0, next_gen],
            )
        })
        .await?;
        b(|| {
            tx.execute(
                //language=SQLite
                "UPDATE snapshots SET files = ? WHERE repo_url = ? AND snapshot_id = ?",
                params![count, &snapshot.repo_url.0, &snapshot.snapshot_id.0],
            )
        })
        .await?;
        drop(stmt);
        b(move || tx.commit()).await?;
        Ok(count)
    }
}

fn prepare_connection(conn: &mut Connection) -> eyre::Result<()> {
    conn.pragma_update(None, "journal_mode", "wal")?;
    conn.pragma_update(None, "synchronous", "normal")?;
    conn.pragma_update(None, "foreign_keys", "on")?;
    Ok(())
}

fn migrations() -> Migrations<'static> {
    Migrations::new(vec![
        //language=SQLite
        M::up(
            r#"CREATE TABLE snapshots
(
    generation  INTEGER NOT NULL,
    repo_url    TEXT    NOT NULL,
    snapshot_id TEXT    NOT NULL,
    backup      TEXT,
    short_id    TEXT    NOT NULL,
    parent      TEXT,
    tree        TEXT    NOT NULL,
    hostname    TEXT    NOT NULL,
    username    TEXT    NOT NULL,
    time        TEXT    NOT NULL,
    tags        TEXT    NOT NULL,
    files       INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (repo_url, snapshot_id)
) STRICT;

CREATE INDEX snapshots_time_idx ON snapshots (time);"#,
        ),
        //language=SQLite
        M::up(
            r#"CREATE TABLE nodes
(
    generation         INTEGER NOT NULL,
    repo_url           TEXT    NOT NULL,
    snapshot_id        TEXT    NOT NULL,
    path               TEXT    NOT NULL,
    name               TEXT    NOT NULL,
    type               TEXT    NOT NULL,
    parent             TEXT,
    uid                INTEGER NOT NULL,
    gid                INTEGER NOT NULL,
    size               INTEGER,
    mode               INTEGER NOT NULL,
    permissions_string TEXT    NOT NULL,
    mtime              TEXT    NOT NULL,
    atime              TEXT    NOT NULL,
    ctime              TEXT    NOT NULL,
    PRIMARY KEY (repo_url, snapshot_id, path),
    FOREIGN KEY (repo_url, snapshot_id) REFERENCES snapshots (repo_url, snapshot_id) ON DELETE CASCADE,
    FOREIGN KEY (repo_url, snapshot_id, parent) REFERENCES nodes (repo_url, snapshot_id, path) ON DELETE CASCADE
) STRICT;

CREATE INDEX nodes_path_idx ON nodes (path);
CREATE INDEX nodes_name_idx ON nodes (name);
CREATE INDEX nodes_parent_idx ON nodes (parent);"#,
        ),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SnapshotId, TreeId};
    use cirrus_core::{config::backup, tag::Tag};
    use time::macros::datetime;

    #[test]
    fn test_migrations() {
        migrations().validate().unwrap();
    }

    fn snapshots1(repo_url: &repo::Url) -> [Snapshot; 2] {
        [
            Snapshot {
                key: SnapshotKey {
                    repo_url: repo_url.clone(),
                    snapshot_id: SnapshotId("1234".to_string()),
                },
                backup: None,
                short_id: "12".to_string(),
                parent: None,
                tree: TreeId("abcd".to_string()),
                hostname: "host1".to_string(),
                username: "user1".to_string(),
                time: datetime!(2022-10-25 20:44:12 +0),
                tags: vec![Tag("tag1".to_string())],
            },
            Snapshot {
                key: SnapshotKey {
                    repo_url: repo_url.clone(),
                    snapshot_id: SnapshotId("5678".to_string()),
                },
                backup: Some(backup::Name("bkp".to_string())),
                short_id: "56".to_string(),
                parent: None,
                tree: TreeId("ef".to_string()),
                hostname: "host2".to_string(),
                username: "user2".to_string(),
                time: datetime!(2022-04-18 10:50:31 +0),
                tags: vec![Tag("tag2".to_string()), Tag("tag3".to_string())],
            },
        ]
    }

    fn snapshots2(repo_url: &repo::Url) -> [Snapshot; 2] {
        [
            Snapshot {
                key: SnapshotKey {
                    repo_url: repo_url.clone(),
                    snapshot_id: SnapshotId("5678".to_string()),
                },
                backup: None,
                short_id: "12".to_string(),
                parent: None,
                tree: TreeId("abcd".to_string()),
                hostname: "host1".to_string(),
                username: "user1".to_string(),
                time: datetime!(2022-10-25 20:44:12 +0),
                tags: vec![Tag("tag1".to_string())],
            },
            Snapshot {
                key: SnapshotKey {
                    repo_url: repo_url.clone(),
                    snapshot_id: SnapshotId("1111".to_string()),
                },
                backup: Some(backup::Name("abc".to_string())),
                short_id: "11".to_string(),
                parent: None,
                tree: TreeId("ef".to_string()),
                hostname: "host3".to_string(),
                username: "user3".to_string(),
                time: datetime!(2020-03-06 09:06:47 +0),
                tags: vec![Tag("tag4".to_string())],
            },
        ]
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn should_save_snapshots() {
        let repo = repo::Definition {
            url: repo::Url("local:/tmp/repo".to_string()),
            ..Default::default()
        };
        let snapshots = snapshots1(&repo.url);
        let tmp = tempfile::tempdir().unwrap();
        let mut db = Database::new(tmp.path().join("test.db")).await.unwrap();

        db.save_snapshots(&repo, &snapshots).await.unwrap();

        let result = db.get_snapshots(&repo).await.unwrap();
        assert_eq!(&result, &snapshots);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn should_save_and_replace_snapshots() {
        let repo = repo::Definition {
            url: repo::Url("local:/tmp/repo".to_string()),
            ..Default::default()
        };
        let snapshots1 = snapshots1(&repo.url);
        let snapshots2 = snapshots2(&repo.url);
        let tmp = tempfile::tempdir().unwrap();
        let mut db = Database::new(tmp.path().join("test.db")).await.unwrap();

        db.save_snapshots(&repo, &snapshots1).await.unwrap();
        db.save_snapshots(&repo, &snapshots2).await.unwrap();

        let result = db.get_snapshots(&repo).await.unwrap();
        assert_eq!(&result, &snapshots2);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn should_save_snapshots_for_multiple_repos() {
        let repo1 = repo::Definition {
            url: repo::Url("local:/tmp/repo1".to_string()),
            ..Default::default()
        };
        let repo2 = repo::Definition {
            url: repo::Url("local:/tmp/repo2".to_string()),
            ..Default::default()
        };
        let snapshots1 = snapshots1(&repo1.url);
        let snapshots2 = snapshots2(&repo2.url);
        let tmp = tempfile::tempdir().unwrap();
        let mut db = Database::new(tmp.path().join("test.db")).await.unwrap();

        db.save_snapshots(&repo1, &snapshots1).await.unwrap();
        db.save_snapshots(&repo2, &snapshots2).await.unwrap();

        let result = db.get_snapshots(&repo1).await.unwrap();
        assert_eq!(&result, &snapshots1);
        let result = db.get_snapshots(&repo2).await.unwrap();
        assert_eq!(&result, &snapshots2);
    }
}
