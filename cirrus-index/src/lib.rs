use cirrus_core::{config::repo, restic::Restic};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use time::OffsetDateTime;
use tokio::task::block_in_place;

mod migrations;
mod restic_json;

// TODO: rename... what module
#[derive(Debug)]
pub struct IndexRepository {
    path: PathBuf,
    conn: Connection,
}

impl IndexRepository {
    pub async fn new(path: impl Into<PathBuf>) -> eyre::Result<Self> {
        let path = path.into();
        let conn = block_in_place(|| Connection::open(&path))?;
        let mut repo = IndexRepository { path, conn };
        repo.prepare_connection().await?;
        migrations::apply_migrations(&mut repo.conn).await?;
        Ok(repo)
    }

    async fn prepare_connection(&mut self) -> eyre::Result<()> {
        block_in_place(|| {
            self.conn.pragma_update(None, "journal_mode", "wal")?;
            self.conn.pragma_update(None, "synchronous", "normal")?;
            self.conn.pragma_update(None, "foreign_keys", "on")?;
            Ok(())
        })
    }

    pub async fn get_snapshots(&mut self, repo: &repo::Definition) -> eyre::Result<Vec<Snapshot>> {
        block_in_place(|| {
            let mut stmt = self
                .conn
                //language=SQLite
                .prepare("SELECT * FROM snapshots WHERE repo_url = ? ORDER BY time DESC")?;
            let rows = stmt.query(&[&repo.url.0])?;
            let snapshots =
                serde_rusqlite::from_rows::<Snapshot>(rows).collect::<Result<_, _>>()?;
            Ok(snapshots)
        })
    }

    async fn save_snapshots(
        &mut self,
        repo: &repo::Definition,
        snapshots: impl IntoIterator<Item = &Snapshot>,
    ) -> eyre::Result<()> {
        block_in_place(|| {
            let tx = self.conn.transaction()?;

            let generation = tx
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
                       id,
                       short_id,
                       parent,
                       tree,
                       hostname,
                       username,
                       time,
                       tags)
VALUES (:generation,
        :repo_url,
        :id,
        :short_id,
        :parent,
        :tree,
        :hostname,
        :username,
        :time,
        :tags)",
            )?;
            for snapshot in snapshots {
                let mut params = serde_rusqlite::to_params_named(snapshot)?;
                params.push((":generation".to_owned(), Box::new(next_gen)));
                stmt.execute(&*params.to_slice())?;
            }
            tx.execute(
                "DELETE FROM snapshots WHERE repo_url = ? AND generation != ?",
                params![&repo.url.0, next_gen],
            )?;
            drop(stmt);
            tx.commit()?;
            Ok(())
        })
    }
}

pub async fn index_snapshots(
    restic: &Restic,
    repository: &mut IndexRepository,
    repo: &repo::Definition,
) -> eyre::Result<()> {
    todo!()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Uid(pub u32);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Gid(pub u32);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FileSize(pub u64);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Type {
    Dir,
    File,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SnapshotId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TreeId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Tag(pub String);

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub repo_url: repo::Url,
    pub id: SnapshotId,
    pub short_id: String,
    pub parent: Option<SnapshotId>,
    pub tree: TreeId,
    pub hostname: String,
    pub username: String,
    #[serde(with = "time::serde::iso8601")]
    pub time: OffsetDateTime,
    #[serde(
        default,
        serialize_with = "serialize_tags",
        deserialize_with = "deserialize_tags"
    )]
    pub tags: Vec<Tag>,
}

fn serialize_tags<S: serde::Serializer>(v: &Vec<Tag>, s: S) -> Result<S::Ok, S::Error> {
    use itertools::Itertools;
    let tag_string = v.iter().map(|s| &s.0).join(",");
    s.serialize_str(&tag_string)
}

fn deserialize_tags<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<Tag>, D::Error> {
    let tag_string = String::deserialize(d)?;
    let tags = tag_string.split(',').map(|s| Tag(s.to_string())).collect();
    Ok(tags)
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    fn snapshots1(repo_url: &repo::Url) -> [Snapshot; 2] {
        [
            Snapshot {
                repo_url: repo_url.clone(),
                id: SnapshotId("1234".to_string()),
                short_id: "12".to_string(),
                parent: None,
                tree: TreeId("abcd".to_string()),
                hostname: "host1".to_string(),
                username: "user1".to_string(),
                time: datetime!(2022-10-25 20:44:12 +0),
                tags: vec![Tag("tag1".to_string())],
            },
            Snapshot {
                repo_url: repo_url.clone(),
                id: SnapshotId("5678".to_string()),
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
                repo_url: repo_url.clone(),
                id: SnapshotId("5678".to_string()),
                short_id: "12".to_string(),
                parent: None,
                tree: TreeId("abcd".to_string()),
                hostname: "host1".to_string(),
                username: "user1".to_string(),
                time: datetime!(2022-10-25 20:44:12 +0),
                tags: vec![Tag("tag1".to_string())],
            },
            Snapshot {
                repo_url: repo_url.clone(),
                id: SnapshotId("1111".to_string()),
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
        let mut store = IndexRepository::new(tmp.path().join("test.db"))
            .await
            .unwrap();

        store.save_snapshots(&repo, &snapshots).await.unwrap();

        let result = store.get_snapshots(&repo).await.unwrap();
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
        let mut store = IndexRepository::new(tmp.path().join("test.db"))
            .await
            .unwrap();

        store.save_snapshots(&repo, &snapshots1).await.unwrap();
        store.save_snapshots(&repo, &snapshots2).await.unwrap();

        let result = store.get_snapshots(&repo).await.unwrap();
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
        let mut store = IndexRepository::new(tmp.path().join("test.db"))
            .await
            .unwrap();

        store.save_snapshots(&repo1, &snapshots1).await.unwrap();
        store.save_snapshots(&repo2, &snapshots2).await.unwrap();

        let result = store.get_snapshots(&repo1).await.unwrap();
        assert_eq!(&result, &snapshots1);
        let result = store.get_snapshots(&repo2).await.unwrap();
        assert_eq!(&result, &snapshots2);
    }
}
