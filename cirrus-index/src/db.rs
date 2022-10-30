use crate::{File, FileId, Snapshot, Tree, TreeHash, TreeId, Version};
use cirrus_core::config::repo;
use futures::{Stream, StreamExt};
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use rusqlite_migration::{Migrations, M};
use std::path::Path;

async fn b<T>(f: impl FnOnce() -> T) -> T {
    tokio::task::block_in_place(f)
}

#[derive(Debug)]
pub struct Database {
    conn: Connection,
}

impl Database {
    pub async fn new(cache_dir: &Path, repo: &repo::Name) -> eyre::Result<Self> {
        let file_path = cache_dir.join(format!("index-{}.sqlite", repo.0));
        let mut conn = b(|| Connection::open(&file_path)).await?;
        b(|| {
            conn.pragma_update(None, "journal_mode", "wal")?;
            conn.pragma_update(None, "synchronous", "normal")?;
            conn.pragma_update(None, "foreign_keys", "on")?;
            Ok::<_, eyre::Report>(())
        })
        .await?;
        b(|| migrations().to_latest(&mut conn)).await?;
        Ok(Database { conn })
    }

    pub async fn get_snapshots(&mut self) -> eyre::Result<Vec<Snapshot>> {
        //language=SQLite
        let mut stmt = b(|| {
            self.conn
                .prepare("SELECT * FROM snapshots ORDER BY time DESC")
        })
        .await?;
        let rows = b(|| stmt.query(())).await?;
        let snapshots = serde_rusqlite::from_rows(rows).collect::<Result<_, _>>()?;
        Ok(snapshots)
    }

    pub(crate) async fn save_snapshots(
        &mut self,
        snapshots: impl IntoIterator<Item = (Tree, Snapshot)>,
    ) -> eyre::Result<u64> {
        let tx = b(|| self.conn.transaction()).await?;
        //language=SQLite
        let db_gen =
            b(|| tx.query_row("SELECT generation FROM snapshots LIMIT 1", (), |r| r.get(0)))
                .await
                .optional()?
                .unwrap_or(0);
        let generation = db_gen + 1;
        let mut count = 0;
        for (tree, mut snapshot) in snapshots {
            let tree_id = Self::get_or_insert_tree(&tx, &tree).await?;
            snapshot.tree = tree_id;
            Self::insert_snapshot(&tx, &snapshot, generation).await?;
            count += 1;
        }
        b(|| {
            tx.execute(
                //language=SQLite
                "DELETE FROM snapshots WHERE generation != ? ",
                [generation],
            )
        })
        .await?;
        b(|| tx.commit()).await?;
        Ok(count)
    }

    pub async fn get_unindexed_snapshots(
        &mut self,
        limit: u64,
    ) -> eyre::Result<Vec<(Tree, Snapshot)>> {
        //language=SQLite
        let mut stmt = self.conn.prepare_cached(
            "SELECT *
FROM snapshots
         JOIN trees ON snapshots.tree = trees.id
WHERE file_count = 0
GROUP BY trees.id
LIMIT ?",
        )?;
        let rows = b(|| stmt.query([limit])).await?;
        let result = rows
            .and_then(|row| {
                let tree = serde_rusqlite::from_row(row)?;
                let snapshot = serde_rusqlite::from_row(row)?;
                Ok::<_, eyre::Report>((tree, snapshot))
            })
            .collect::<Result<_, _>>()?;
        Ok(result)
    }

    pub(crate) async fn save_files(
        &mut self,
        tree: &Tree,
        files: impl Stream<Item = eyre::Result<(File, Version)>>,
    ) -> eyre::Result<u64> {
        let tx = b(|| self.conn.transaction()).await?;
        let mut count = 0;
        tokio::pin!(files);
        while let Some(file_and_version) = files.next().await {
            let (file, mut version) = file_and_version?;
            let file_id = Self::get_or_insert_file(&tx, &file).await?;
            version.file = file_id;
            Self::insert_version(&tx, &version).await?;
            count += 1;
        }

        //language=SQLite
        b(|| {
            tx.execute(
                "UPDATE trees SET file_count = ? WHERE id = ?",
                params![count, tree.id.0],
            )
        })
        .await?;
        b(|| tx.commit()).await?;
        Ok(count)
    }

    async fn get_or_insert_tree(tx: &Transaction<'_>, tree: &Tree) -> eyre::Result<TreeId> {
        //language=SQLite
        let mut get_stmt = tx.prepare_cached("SELECT id FROM trees WHERE hash = ?")?;
        //language=SQLite
        let mut insert_stmt = tx.prepare_cached(
            "INSERT INTO trees (hash, file_count) VALUES (:hash, :file_count) RETURNING id",
        )?;
        let id = b(|| get_stmt.query_row([&tree.hash.0], |r| r.get(0)))
            .await
            .optional()?;
        let id = match id {
            Some(id) => id,
            None => {
                let params = serde_rusqlite::to_params_named(tree)?;
                b(|| insert_stmt.query_row(&*params.to_slice(), |r| r.get(0))).await?
            }
        };
        Ok(TreeId(id))
    }

    async fn insert_snapshot(
        tx: &Transaction<'_>,
        snapshot: &Snapshot,
        generation: u64,
    ) -> eyre::Result<()> {
        //language=SQLite
        let mut stmt = tx.prepare(
            "INSERT OR
REPLACE
INTO snapshots(generation,
               snapshot_id,
               backup,
               parent,
               tree,
               hostname,
               username,
               time,
               tags)
VALUES (:generation,
        :snapshot_id,
        :backup,
        :parent,
        :tree,
        :hostname,
        :username,
        :time,
        :tags)",
        )?;
        let mut params = serde_rusqlite::to_params_named(snapshot)?;
        params.push((":generation".to_owned(), Box::new(generation)));
        b(|| stmt.execute(&*params.to_slice())).await?;
        Ok(())
    }

    async fn get_or_insert_file(tx: &Transaction<'_>, file: &File) -> eyre::Result<FileId> {
        //language=SQLite
        let mut get_stmt = tx.prepare_cached("SELECT id FROM files WHERE path = ?")?;
        //language=SQLite
        let mut insert_stmt = tx.prepare_cached(
            "INSERT INTO files (path, parent, name) VALUES (:path, :parent, :name) RETURNING id",
        )?;
        let id = b(|| get_stmt.query_row([&file.path], |r| r.get(0)))
            .await
            .optional()?;
        let id = match id {
            Some(id) => id,
            None => {
                let params = serde_rusqlite::to_params_named(file)?;
                b(|| insert_stmt.query_row(&*params.to_slice(), |r| r.get(0))).await?
            }
        };
        Ok(FileId(id))
    }

    async fn insert_version(tx: &Transaction<'_>, version: &Version) -> eyre::Result<()> {
        //language=SQLite
        let mut stmt = tx.prepare_cached("INSERT INTO files_versions (file, tree, type, uid, gid, size, mode, permissions_string, mtime, atime, ctime)
VALUES (:file, :tree, :type, :uid, :gid, :size, :mode, :permissions_string, :mtime, :atime, :ctime)
ON CONFLICT(file, tree) DO NOTHING")?;
        let params = serde_rusqlite::to_params_named(version)?;
        b(|| stmt.execute(&*params.to_slice())).await?;
        Ok(())
    }
}

fn migrations() -> Migrations<'static> {
    Migrations::new(vec![
        //language=SQLite
        M::up(
            r#"CREATE TABLE snapshots
(
    generation  INTEGER NOT NULL,
    snapshot_id TEXT    NOT NULL PRIMARY KEY,
    backup      TEXT,
    parent      TEXT,
    tree        INTEGER NOT NULL,
    hostname    TEXT    NOT NULL,
    username    TEXT    NOT NULL,
    time        TEXT    NOT NULL,
    tags        TEXT    NOT NULL,
    FOREIGN KEY (tree) REFERENCES trees (id)
) STRICT;

CREATE INDEX snapshots_time_idx ON snapshots (time);

CREATE TABLE trees
(
    id         INTEGER PRIMARY KEY,
    hash       TEXT    NOT NULL UNIQUE,
    file_count INTEGER NOT NULL
) STRICT;
"#,
        ),
        //language=SQLite
        M::up(
            r#"CREATE TABLE files
(
    id     INTEGER PRIMARY KEY,
    path   TEXT NOT NULL UNIQUE,
    parent TEXT,
    name   TEXT NOT NULL
) STRICT;

CREATE TABLE files_versions
(
    file               INTEGER NOT NULL,
    tree               INTEGER NOT NULL,
    type               INTEGER NOT NULL,
    uid                INTEGER NOT NULL,
    gid                INTEGER NOT NULL,
    size               INTEGER,
    mode               INTEGER NOT NULL,
    permissions_string TEXT    NOT NULL,
    mtime              INTEGER NOT NULL,
    atime              INTEGER NOT NULL,
    ctime              INTEGER NOT NULL,
    PRIMARY KEY (file, tree),
    FOREIGN KEY (file) REFERENCES files (id) ON DELETE CASCADE ON UPDATE CASCADE,
    FOREIGN KEY (tree) REFERENCES trees (id) ON DELETE CASCADE ON UPDATE CASCADE
) STRICT;
"#,
        ),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SnapshotId, TreeHash};
    use cirrus_core::{config::backup, tag::Tag};
    use time::macros::datetime;

    #[test]
    fn test_migrations() {
        migrations().validate().unwrap();
    }

    fn snapshots1() -> [(Tree, Snapshot); 2] {
        [
            (
                Tree {
                    id: TreeId::default(),
                    hash: TreeHash("abcd".to_string()),
                    file_count: 15,
                },
                Snapshot {
                    snapshot_id: SnapshotId("1234".to_string()),
                    backup: None,
                    parent: None,
                    tree: TreeId::default(),
                    hostname: "host1".to_string(),
                    username: "user1".to_string(),
                    time: datetime!(2022-10-25 20:44:12 +0),
                    tags: vec![Tag("tag1".to_string())],
                },
            ),
            (
                Tree {
                    id: TreeId::default(),
                    hash: TreeHash("ef".to_string()),
                    file_count: 0,
                },
                Snapshot {
                    snapshot_id: SnapshotId("5678".to_string()),
                    backup: Some(backup::Name("bkp".to_string())),
                    parent: None,
                    tree: TreeId::default(),
                    hostname: "host2".to_string(),
                    username: "user2".to_string(),
                    time: datetime!(2022-04-18 10:50:31 +0),
                    tags: vec![Tag("tag2".to_string()), Tag("tag3".to_string())],
                },
            ),
        ]
    }

    fn snapshots2() -> [(Tree, Snapshot); 2] {
        [
            (
                Tree {
                    id: TreeId::default(),
                    hash: TreeHash("abcd".to_string()),
                    file_count: 7,
                },
                Snapshot {
                    snapshot_id: SnapshotId("5678".to_string()),
                    backup: None,
                    parent: None,
                    tree: TreeId::default(),
                    hostname: "host1".to_string(),
                    username: "user1".to_string(),
                    time: datetime!(2022-10-25 20:44:12 +0),
                    tags: vec![Tag("tag1".to_string())],
                },
            ),
            (
                Tree {
                    id: TreeId::default(),
                    hash: TreeHash("ef".to_string()),
                    file_count: 0,
                },
                Snapshot {
                    snapshot_id: SnapshotId("1111".to_string()),
                    backup: Some(backup::Name("abc".to_string())),
                    parent: None,
                    tree: TreeId::default(),
                    hostname: "host3".to_string(),
                    username: "user3".to_string(),
                    time: datetime!(2020-03-06 09:06:47 +0),
                    tags: vec![Tag("tag4".to_string())],
                },
            ),
        ]
    }

    fn test_repo() -> repo::Name {
        repo::Name("test".to_string())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn should_save_snapshots() {
        let snapshots = snapshots1();
        let tmp = tempfile::tempdir().unwrap();
        let mut db = Database::new(tmp.path(), &test_repo()).await.unwrap();

        db.save_snapshots(snapshots.clone()).await.unwrap();

        let result = db.get_snapshots().await.unwrap();
        //assert_eq!(&result, &snapshots);
        todo!()
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn should_save_and_replace_snapshots() {
        let snapshots1 = snapshots1();
        let snapshots2 = snapshots2();
        let tmp = tempfile::tempdir().unwrap();
        let mut db = Database::new(tmp.path(), &test_repo()).await.unwrap();

        db.save_snapshots(snapshots1.clone()).await.unwrap();
        db.save_snapshots(snapshots2.clone()).await.unwrap();

        let result = db.get_snapshots().await.unwrap();
        //assert_eq!(&result, &snapshots2);
        todo!()
    }
}
