use crate::{File, FileSnapshotMeta, Parent, Snapshot, Version};
use cirrus_core::config::repo;
use futures::{Stream, StreamExt};
use rusqlite::{params, CachedStatement, Connection, OptionalExtension, Transaction};
use std::path::Path;
use time::OffsetDateTime;

#[derive(Debug, PartialEq, Eq, Copy, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
struct FileId(i64);

#[derive(Debug, PartialEq, Eq, Copy, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
struct TreeId(i64);

#[derive(Debug, PartialEq, Eq, Copy, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
struct VersionId(i64);

#[derive(Debug)]
pub struct Database {
    conn: Connection,
}

impl Database {
    pub async fn new(cache_dir: &Path, repo: &repo::Name) -> eyre::Result<Self> {
        tokio::task::block_in_place(|| Database::new_sync(cache_dir, repo))
    }

    fn new_sync(cache_dir: &Path, repo: &repo::Name) -> eyre::Result<Self> {
        let file_path = cache_dir.join(format!("index-{}.sqlite", repo.0));
        let mut conn = Connection::open(&file_path)?;
        conn.pragma_update(None, "journal_mode", "wal")?;
        conn.pragma_update(None, "synchronous", "normal")?;
        conn.pragma_update(None, "foreign_keys", "on")?;
        migrations().to_latest(&mut conn)?;
        Ok(Database { conn })
    }

    pub async fn get_snapshots(&mut self) -> eyre::Result<Vec<Snapshot>> {
        tokio::task::block_in_place(|| self.get_snapshots_sync())
    }

    fn get_snapshots_sync(&mut self) -> eyre::Result<Vec<Snapshot>> {
        //language=SQLite
        let mut stmt = self
            .conn
            .prepare_cached("SELECT * FROM snapshots ORDER BY time DESC")?;
        let rows = stmt.query(())?;
        let snapshots = serde_rusqlite::from_rows(rows).collect::<Result<_, _>>()?;
        Ok(snapshots)
    }

    pub async fn get_unindexed_snapshots(
        &mut self,
        newer_than: OffsetDateTime,
    ) -> eyre::Result<Vec<Snapshot>> {
        tokio::task::block_in_place(|| self.get_unindexed_snapshots_sync(newer_than))
    }

    fn get_unindexed_snapshots_sync(
        &mut self,
        newer_than: OffsetDateTime,
    ) -> eyre::Result<Vec<Snapshot>> {
        //language=SQLite
        let mut stmt = self.conn.prepare_cached(
            "--
SELECT snapshots.*
FROM snapshots
         LEFT JOIN trees ON snapshots.tree_hash = trees.hash
WHERE (file_count IS NULL OR file_count = 0)
  AND snapshots.time >= ?
GROUP BY snapshots.tree_hash
ORDER BY MAX(snapshots.time) DESC",
        )?;
        let rows = stmt.query([newer_than.unix_timestamp()])?;
        let snapshots = serde_rusqlite::from_rows(rows).collect::<Result<_, _>>()?;
        Ok(snapshots)
    }

    pub async fn get_unindexed_snapshots_number(
        &mut self,
        limit: u64,
    ) -> eyre::Result<Vec<Snapshot>> {
        //language=SQLite
        let mut stmt = self.conn.prepare_cached(
            "--
SELECT snapshots.*
FROM snapshots
         LEFT JOIN trees ON snapshots.tree_hash = trees.hash
WHERE file_count IS NULL
   OR file_count = 0
GROUP BY snapshots.tree_hash
ORDER BY MAX(snapshots.time) DESC
LIMIT ?",
        )?;
        let rows = stmt.query([limit])?;
        let snapshots = serde_rusqlite::from_rows(rows).collect::<Result<_, _>>()?;
        Ok(snapshots)
    }

    pub async fn get_files(
        &mut self,
        parent: &Parent,
        limit: u64,
    ) -> eyre::Result<Vec<(File, Version, FileSnapshotMeta)>> {
        tokio::task::block_in_place(|| self.get_files_sync(parent, limit))
    }

    fn get_files_sync(
        &mut self,
        parent: &Parent,
        limit: u64,
    ) -> eyre::Result<Vec<(File, Version, FileSnapshotMeta)>> {
        #[derive(serde::Serialize)]
        struct Params<'a> {
            parent: &'a Parent,
            limit: u64,
        }

        #[derive(serde::Deserialize)]
        struct RowResult {
            #[serde(flatten)]
            file: File,
            #[serde(flatten)]
            version: Version,
            #[serde(flatten)]
            snapshot_meta: FileSnapshotMeta,
        }

        //language=SQLite
        let mut stmt = self.conn.prepare_cached(
            "--
SELECT max(snapshots.time) AS _selector,
       files.*,
       file_versions.*,
       snapshots.snapshot_id,
       snapshots.hostname,
       snapshots.time
FROM files
         JOIN file_versions ON file_versions.file = files.id
         JOIN version_tree_map ON version_tree_map.version = file_versions.id
         JOIN trees ON trees.id = version_tree_map.tree
         JOIN snapshots ON snapshots.tree_hash = trees.hash
WHERE files.parent = :parent
GROUP BY files.id
ORDER BY files.type, files.name
LIMIT :limit",
        )?;
        let params = serde_rusqlite::to_params_named(Params { parent, limit })?;
        let rows = stmt.query(&*params.to_slice())?;
        let files = serde_rusqlite::from_rows::<RowResult>(rows)
            .map(|row| {
                let row = row?;
                Ok((row.file, row.version, row.snapshot_meta))
            })
            .collect::<Result<_, eyre::Report>>()?;
        Ok(files)
    }

    pub async fn import_snapshots(
        &mut self,
        snapshots: impl IntoIterator<Item = Snapshot>,
    ) -> eyre::Result<u64> {
        tokio::task::block_in_place(|| self.import_snapshots_sync(snapshots))
    }

    fn import_snapshots_sync(
        &mut self,
        snapshots: impl IntoIterator<Item = Snapshot>,
    ) -> eyre::Result<u64> {
        let tx = self.conn.transaction()?;
        //language=SQLite
        let prev_gen = tx
            .query_row("SELECT generation FROM snapshots LIMIT 1", (), |r| r.get(0))
            .optional()?
            .unwrap_or(0);
        let generation = prev_gen + 1;
        let mut count = 0;
        for snapshot in snapshots {
            insert_snapshot(&tx, &snapshot, generation)?;
            count += 1;
        }
        tx.execute(
            //language=SQLite
            "DELETE FROM snapshots WHERE generation != ? ",
            [generation],
        )?;
        tx.commit()?;
        Ok(count)
    }

    pub async fn import_files(
        &mut self,
        snapshot: &Snapshot,
        files: impl Stream<Item = eyre::Result<(File, Version)>>,
    ) -> eyre::Result<u64> {
        tokio::task::block_in_place(|| self.import_files_sync(snapshot, files))
    }

    fn import_files_sync(
        &mut self,
        snapshot: &Snapshot,
        files: impl Stream<Item = eyre::Result<(File, Version)>>,
    ) -> eyre::Result<u64> {
        let tx = self.conn.transaction()?;
        let tree_id = insert_tree(&tx, snapshot)?;
        let mut count = 0;
        tokio::pin!(files);
        let rt = tokio::runtime::Handle::current();
        while let Some(file_and_version) = rt.block_on(files.next()) {
            let (file, version) = file_and_version?;
            let file_id = upsert_file(&tx, &file)?;
            let version_id = upsert_version(&tx, file_id, &version)?;
            insert_version_tree_map(&tx, version_id, tree_id)?;
            count += 1;
        }
        //language=SQLite
        tx.execute(
            "UPDATE trees SET file_count = ? WHERE id = ?",
            params![count, tree_id.0],
        )?;
        tx.commit()?;
        Ok(count)
    }
}

fn insert_snapshot(tx: &Transaction<'_>, snapshot: &Snapshot, generation: u64) -> eyre::Result<()> {
    #[derive(serde::Serialize)]
    struct Insert<'a> {
        generation: u64,
        #[serde(flatten)]
        snapshot: &'a Snapshot,
    }

    //language=SQLite
    let mut stmt = tx.prepare(
        "--
INSERT OR
REPLACE
INTO snapshots(generation,
               snapshot_id,
               backup,
               parent,
               tree_hash,
               hostname,
               username,
               time,
               tags)
VALUES (:generation,
        :snapshot_id,
        :backup,
        :parent,
        :tree_hash,
        :hostname,
        :username,
        :time,
        :tags)",
    )?;
    let params = serde_rusqlite::to_params_named(Insert {
        generation,
        snapshot,
    })?;
    stmt.execute(&*params.to_slice())?;
    Ok(())
}

fn insert_tree(tx: &Transaction<'_>, snapshot: &Snapshot) -> eyre::Result<TreeId> {
    //language=SQLite
    let mut delete_stmt = tx.prepare_cached("DELETE FROM trees WHERE hash = ?")?;
    //language=SQLite
    let mut stmt =
        tx.prepare_cached("INSERT INTO trees (hash, file_count) VALUES (?, 0) RETURNING id")?;
    delete_stmt.execute([&snapshot.tree_hash.0])?;
    let id = stmt.query_row([&snapshot.tree_hash.0], |r| r.get(0))?;
    Ok(TreeId(id))
}

fn upsert_file(tx: &Transaction<'_>, file: &File) -> eyre::Result<FileId> {
    //language=SQLite
    let get_stmt = tx.prepare_cached(
        "SELECT id FROM files WHERE parent = :parent AND name = :name AND type = :type",
    )?;
    //language=SQLite
    let insert_stmt = tx.prepare_cached(
        "INSERT INTO files (parent, name, type) VALUES (:parent, :name, :type) RETURNING id",
    )?;
    let params = serde_rusqlite::to_params_named(file)?;
    Ok(FileId(upsert(get_stmt, insert_stmt, params)?))
}

fn upsert_version(
    tx: &Transaction<'_>,
    file: FileId,
    version: &Version,
) -> eyre::Result<VersionId> {
    #[derive(serde::Serialize)]
    struct Insert<'a> {
        file: FileId,
        #[serde(flatten)]
        version: &'a Version,
    }

    //language=SQLite
    let get_stmt = tx.prepare_cached(
        "--
SELECT id
FROM file_versions
WHERE file = :file
  AND uid = :uid
  AND gid = :gid
  AND size = :size
  AND mode = :mode
  AND mtime = :mtime
  AND ctime = :ctime",
    )?;
    //language=SQLite
    let insert_stmt = tx.prepare_cached(
        "--
INSERT INTO file_versions (file, uid, gid, size, mode, mtime, ctime)
VALUES (:file, :uid, :gid, :size, :mode, :mtime, :ctime)
RETURNING id",
    )?;
    let params = serde_rusqlite::to_params_named(Insert { file, version })?;
    Ok(VersionId(upsert(get_stmt, insert_stmt, params)?))
}

fn insert_version_tree_map(
    tx: &Transaction<'_>,
    version: VersionId,
    tree: TreeId,
) -> eyre::Result<()> {
    //language=SQLite
    let mut stmt =
        tx.prepare_cached("INSERT INTO version_tree_map (version, tree) VALUES (?, ?);")?;
    stmt.execute([version.0, tree.0])?;
    Ok(())
}

fn upsert(
    mut get_stmt: CachedStatement<'_>,
    mut insert_stmt: CachedStatement<'_>,
    params: serde_rusqlite::NamedParamSlice,
) -> eyre::Result<i64> {
    let id = get_stmt
        .query_row(&*params.to_slice(), |r| r.get(0))
        .optional()?;
    let id = match id {
        Some(id) => id,
        None => insert_stmt.query_row(&*params.to_slice(), |r| r.get(0))?,
    };
    Ok(id)
}

fn migrations() -> rusqlite_migration::Migrations<'static> {
    use rusqlite_migration::{Migrations, M};
    Migrations::new(vec![
        //language=SQLite
        M::up(
            r#"--
CREATE TABLE snapshots
(
    generation  INTEGER NOT NULL,
    snapshot_id TEXT    NOT NULL PRIMARY KEY,
    backup      TEXT,
    parent      TEXT,
    tree_hash   TEXT    NOT NULL,
    hostname    TEXT    NOT NULL,
    username    TEXT    NOT NULL,
    time        INTEGER NOT NULL,
    tags        TEXT    NOT NULL
) STRICT;

CREATE INDEX snapshots_time_idx ON snapshots (time);

CREATE TABLE trees
(
    id         INTEGER PRIMARY KEY,
    hash       TEXT    NOT NULL UNIQUE,
    file_count INTEGER NOT NULL
) STRICT;

CREATE TABLE files
(
    id     INTEGER PRIMARY KEY,
    parent TEXT    NOT NULL,
    name   TEXT    NOT NULL,
    type   INTEGER NOT NULL
) STRICT;

CREATE UNIQUE INDEX files_uniq_idx ON files (parent, name, type);

CREATE TABLE file_versions
(
    id    INTEGER PRIMARY KEY,
    file  INTEGER NOT NULL,
    uid   INTEGER NOT NULL,
    gid   INTEGER NOT NULL,
    size  INTEGER,
    mode  INTEGER NOT NULL,
    mtime INTEGER NOT NULL,
    ctime INTEGER NOT NULL,
    FOREIGN KEY (file) REFERENCES files (id) ON DELETE CASCADE ON UPDATE CASCADE
) STRICT;

CREATE INDEX file_versions_uniq_idx ON file_versions (file, uid, gid, size, mode, mtime, ctime);

CREATE TABLE version_tree_map
(
    version INTEGER NOT NULL,
    tree    INTEGER NOT NULL,
    PRIMARY KEY (version, tree),
    FOREIGN KEY (version) REFERENCES file_versions (id) ON DELETE CASCADE ON UPDATE CASCADE,
    FOREIGN KEY (tree) REFERENCES trees (id) ON DELETE CASCADE ON UPDATE CASCADE
) STRICT;
"#,
        ),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Gid, Mode, Owner, Parent, SnapshotId, TreeHash, Type, Uid};
    use cirrus_core::{config::backup, tag::Tag};
    use std::time::Duration;
    use time::macros::datetime;
    use tokio_stream::StreamExt;

    fn test_repo() -> repo::Name {
        repo::Name("test".to_string())
    }

    fn test_snapshot() -> Snapshot {
        Snapshot {
            snapshot_id: SnapshotId(Default::default()),
            backup: None,
            parent: None,
            tree_hash: TreeHash("12345678".to_string()),
            hostname: Default::default(),
            username: Default::default(),
            time: datetime!(2022-10-25 20:44:12 +0),
            tags: Default::default(),
        }
    }

    fn snapshots1() -> [Snapshot; 2] {
        [
            Snapshot {
                snapshot_id: SnapshotId("1234".to_string()),
                backup: None,
                parent: None,
                tree_hash: TreeHash("abcd".to_string()),
                hostname: "host1".to_string(),
                username: "user1".to_string(),
                time: datetime!(2022-10-25 20:44:12 +0),
                tags: vec![Tag("tag1".to_string())],
            },
            Snapshot {
                snapshot_id: SnapshotId("5678".to_string()),
                backup: Some(backup::Name("bkp".to_string())),
                parent: None,
                tree_hash: TreeHash("ef".to_string()),
                hostname: "host2".to_string(),
                username: "user2".to_string(),
                time: datetime!(2022-04-18 10:50:31 +0),
                tags: vec![Tag("tag2".to_string()), Tag("tag3".to_string())],
            },
        ]
    }

    fn snapshots2() -> [Snapshot; 2] {
        [
            Snapshot {
                snapshot_id: SnapshotId("5678".to_string()),
                backup: None,
                parent: None,
                tree_hash: TreeHash("abcd".to_string()),
                hostname: "host1".to_string(),
                username: "user1".to_string(),
                time: datetime!(2022-10-25 20:44:12 +0),
                tags: vec![Tag("tag1".to_string())],
            },
            Snapshot {
                snapshot_id: SnapshotId("1111".to_string()),
                backup: Some(backup::Name("abc".to_string())),
                parent: None,
                tree_hash: TreeHash("ef".to_string()),
                hostname: "host3".to_string(),
                username: "user3".to_string(),
                time: datetime!(2020-03-06 09:06:47 +0),
                tags: vec![Tag("tag4".to_string())],
            },
        ]
    }

    fn files1() -> [(File, Version); 2] {
        [
            (
                File {
                    parent: Parent(None),
                    name: "tmp".to_string(),
                    r#type: Type::Dir,
                },
                Version {
                    owner: Owner {
                        uid: Uid(1000),
                        gid: Gid(1000),
                    },
                    size: None,
                    mode: Mode(0o755),
                    mtime: datetime!(2022-10-10 10:10:10 +2),
                    ctime: datetime!(2022-10-10 08:10:10 +0),
                },
            ),
            (
                File {
                    parent: Parent(Some("/tmp".to_string())),
                    name: "test".to_string(),
                    r#type: Type::File,
                },
                Version {
                    owner: Owner {
                        uid: Uid(1001),
                        gid: Gid(1001),
                    },
                    size: None,
                    mode: Mode(0o400),
                    mtime: datetime!(2022-11-11 10:10:10 +2),
                    ctime: datetime!(2022-11-11 08:10:10 +0),
                },
            ),
        ]
    }

    #[test]
    fn test_migrations() {
        migrations().validate().unwrap();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn should_save_snapshots() {
        let snapshots = snapshots1();
        let tmp = tempfile::tempdir().unwrap();
        let mut db = Database::new(tmp.path(), &test_repo()).await.unwrap();

        db.import_snapshots(snapshots.clone()).await.unwrap();

        let result = db.get_snapshots().await.unwrap();
        assert_eq!(&result, &snapshots);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn should_save_and_replace_snapshots() {
        let snapshots1 = snapshots1();
        let snapshots2 = snapshots2();
        let tmp = tempfile::tempdir().unwrap();
        let mut db = Database::new(tmp.path(), &test_repo()).await.unwrap();

        db.import_snapshots(snapshots1.clone()).await.unwrap();
        db.import_snapshots(snapshots2.clone()).await.unwrap();

        let result = db.get_snapshots().await.unwrap();
        assert_eq!(&result, &snapshots2);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn should_get_unindexed_snapshots_limit() {
        let snapshots = snapshots1();
        let tmp = tempfile::tempdir().unwrap();
        let mut db = Database::new(tmp.path(), &test_repo()).await.unwrap();

        db.import_snapshots(snapshots.clone()).await.unwrap();

        let result = db.get_unindexed_snapshots_number(10).await.unwrap();
        assert_eq!(&result, &snapshots);
        let result = db.get_unindexed_snapshots_number(1).await.unwrap();
        assert_eq!(&result, &snapshots[..1]);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn should_get_unindexed_snapshots() {
        let snapshots = snapshots1();
        let tmp = tempfile::tempdir().unwrap();
        let mut db = Database::new(tmp.path(), &test_repo()).await.unwrap();

        db.import_snapshots(snapshots.clone()).await.unwrap();

        let result = db
            .get_unindexed_snapshots(OffsetDateTime::UNIX_EPOCH)
            .await
            .unwrap();
        assert_eq!(&result, &snapshots);
        let result = db
            .get_unindexed_snapshots(snapshots[1].time + Duration::from_secs(1))
            .await
            .unwrap();
        assert_eq!(&result, &snapshots[..1]);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn should_save_files() {
        let files_and_versions = files1();
        let snapshot = test_snapshot();
        let tmp = tempfile::tempdir().unwrap();
        let mut db = Database::new(tmp.path(), &test_repo()).await.unwrap();

        db.import_snapshots([snapshot.clone()]).await.unwrap();
        db.import_files(
            &snapshot,
            futures::stream::iter(files_and_versions.clone()).map(Ok),
        )
        .await
        .unwrap();

        let result = db.get_files(&Parent(None), 10).await.unwrap();
        assert_eq!(
            &result,
            &[(
                files_and_versions[0].0.clone(),
                files_and_versions[0].1.clone(),
                snapshot.clone().into(),
            )]
        );
        let result = db
            .get_files(&Parent(Some("/tmp".to_string())), 10)
            .await
            .unwrap();
        assert_eq!(
            &result,
            &[(
                files_and_versions[1].0.clone(),
                files_and_versions[1].1.clone(),
                snapshot.clone().into(),
            )]
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn should_save_duplicated_files() {
        let files_and_versions = files1();
        let snapshot1 = test_snapshot();
        let snapshot2 = Snapshot {
            tree_hash: TreeHash("testhash2".to_string()),
            time: snapshot1.time.replace_year(2023).unwrap(),
            ..test_snapshot()
        };
        let tmp = tempfile::tempdir().unwrap();
        let mut db = Database::new(tmp.path(), &test_repo()).await.unwrap();

        db.import_snapshots([snapshot1.clone(), snapshot2.clone()])
            .await
            .unwrap();
        db.import_files(
            &snapshot1,
            futures::stream::iter(files_and_versions.clone()).map(Ok),
        )
        .await
        .unwrap();
        db.import_files(
            &snapshot2,
            futures::stream::iter(files_and_versions.clone()).map(Ok),
        )
        .await
        .unwrap();

        let result = db.get_files(&Parent(None), 10).await.unwrap();
        assert_eq!(
            &result,
            &[(
                files_and_versions[0].0.clone(),
                files_and_versions[0].1.clone(),
                snapshot2.clone().into(),
            )]
        );
        let result = db
            .get_files(&Parent(Some("/tmp".to_string())), 10)
            .await
            .unwrap();
        assert_eq!(
            &result,
            &[(
                files_and_versions[1].0.clone(),
                files_and_versions[1].1.clone(),
                snapshot2.clone().into(),
            )]
        );
    }
}
