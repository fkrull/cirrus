use crate::{File, Snapshot, TreeId, Version};
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
        //language=SQLite
        let mut stmt =b(||
            self
                .conn
                .prepare("SELECT snapshots.* FROM snapshots LEFT JOIN tree_indexed USING (tree_id) WHERE repo_url = ? AND files_count IS NULL ORDER BY time DESC LIMIT ?")).await?;
        let rows = b(|| stmt.query(params![&repo.url.0, limit])).await?;
        let snapshots = serde_rusqlite::from_rows(rows).collect::<Result<_, _>>()?;
        Ok(snapshots)
    }

    pub(crate) async fn save_snapshots(
        &mut self,
        repo: &repo::Definition,
        snapshots: impl IntoIterator<Item = impl Borrow<Snapshot>>,
    ) -> eyre::Result<u64> {
        let tx = b(|| self.conn.transaction()).await?;

        let generation = b(|| {
            tx
                //language=SQLite
                .query_row("SELECT generation FROM snapshots LIMIT 1", [], |r| r.get(0))
        })
        .await
        .optional()?
        .unwrap_or(0);
        let next_gen = generation + 1;
        let mut stmt = b(|| {
            tx.prepare(
                //language=SQLite
                "INSERT OR
REPLACE
INTO snapshots(generation,
               repo_url,
               snapshot_id,
               backup,
               short_id,
               parent,
               tree_id,
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
        :tree_id,
        :hostname,
        :username,
        :time,
        :tags)",
            )
        })
        .await?;
        let mut count = 0;
        for snapshot in snapshots {
            let snapshot = snapshot.borrow();
            let mut params = serde_rusqlite::to_params_named(snapshot)?;
            params.push((":generation".to_owned(), Box::new(next_gen)));
            b(|| stmt.execute(&*params.to_slice())).await?;
            count += 1;
        }
        b(|| {
            tx.execute(
                //language=SQLite
                "DELETE FROM snapshots WHERE repo_url = ? AND generation != ? ",
                params![&repo.url.0, next_gen],
            )
        })
        .await?;
        drop(stmt);
        b(|| tx.commit()).await?;
        Ok(count)
    }

    pub(crate) async fn save_files(
        &mut self,
        tree_id: &TreeId,
        files: impl Stream<Item = eyre::Result<(File, Version)>>,
    ) -> eyre::Result<u64> {
        let tx = b(|| self.conn.transaction()).await?;
        //language=SQLite
        let mut get_file_stmt =
            b(|| tx.prepare("SELECT id FROM files WHERE repo_url = ? AND path = ?")).await?;
        //language=SQLite
        let mut insert_file_stmt = b(|| {
            tx.prepare(
                "INSERT INTO files(repo_url, path, parent, name)
VALUES (:repo_url, :path, :parent, :name)
RETURNING id",
            )
        })
        .await?;
        //language=SQLite
        let mut insert_version_stmt = b(|| tx.prepare(
            "INSERT INTO files_versions (file,
                            tree_id,
                            type,
                            uid,
                            gid,
                            size,
                            mode,
                            permissions_string,
                            mtime,
                            atime,
                            ctime)
VALUES (:file, :tree_id, :type, :uid, :gid, :size, :mode, :permissions_string, :mtime, :atime, :ctime) "
        )).await?;

        let mut count = 0;
        tokio::pin!(files);
        while let Some(file_and_version) = files.next().await {
            let (file, mut version) = file_and_version?;
            let id =
                b(|| get_file_stmt.query_row(params![&file.repo_url.0, &file.path], |r| r.get(0)))
                    .await
                    .optional()?;
            let id: u64 = match id {
                Some(id) => id,
                None => {
                    let params = serde_rusqlite::to_params_named(file)?;
                    b(|| insert_file_stmt.query_row(&*params.to_slice(), |r| r.get(0))).await?
                }
            };
            version.file = id;
            let params = serde_rusqlite::to_params_named(version)?;
            b(|| insert_version_stmt.execute(&*params.to_slice())).await?;
            count += 1;
        }

        //language=SQLite
        b(|| {
            tx.execute(
                "INSERT INTO tree_indexed (tree_id, files_count) VALUES (?, ?)",
                params![&tree_id.0, count],
            )
        })
        .await?;

        drop(get_file_stmt);
        drop(insert_file_stmt);
        drop(insert_version_stmt);
        b(|| tx.commit()).await?;
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
    tree_id     TEXT    NOT NULL,
    hostname    TEXT    NOT NULL,
    username    TEXT    NOT NULL,
    time        TEXT    NOT NULL,
    tags        TEXT    NOT NULL,
    PRIMARY KEY (repo_url, snapshot_id)
) STRICT;

CREATE INDEX snapshots_time_idx ON snapshots (time);"#,
        ),
        //language=SQLite
        M::up(
            r#"CREATE TABLE files
(
    id       INTEGER PRIMARY KEY,
    repo_url TEXT NOT NULL,
    path     TEXT NOT NULL,
    parent   TEXT,
    name     TEXT NOT NULL
) STRICT;

CREATE UNIQUE INDEX files_uniq_idx ON files (repo_url, path);

CREATE TABLE files_versions
(
    file               INTEGER NOT NULL,
    tree_id            TEXT    NOT NULL,
    type               TEXT    NOT NULL,
    uid                INTEGER NOT NULL,
    gid                INTEGER NOT NULL,
    size               INTEGER,
    mode               INTEGER NOT NULL,
    permissions_string TEXT    NOT NULL,
    mtime              INTEGER NOT NULL,
    atime              INTEGER NOT NULL,
    ctime              INTEGER NOT NULL,
    PRIMARY KEY (file, tree_id),
    FOREIGN KEY (file) REFERENCES files (id) ON DELETE CASCADE ON UPDATE CASCADE
) STRICT;

CREATE TABLE tree_indexed
(
    tree_id     TEXT    NOT NULL PRIMARY KEY,
    files_count INTEGER NOT NULL
) STRICT;"#,
        ),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SnapshotId, SnapshotKey, TreeId};
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
                tree_id: TreeId("abcd".to_string()),
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
                tree_id: TreeId("ef".to_string()),
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
                tree_id: TreeId("abcd".to_string()),
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
                tree_id: TreeId("ef".to_string()),
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
