use crate::{Database, FileSize, Gid, Snapshot, SnapshotId, TreeId, Type, Uid};
use cirrus_core::{
    config::{backup, repo},
    restic::{Options, Output, Restic},
    secrets::RepoWithSecrets,
    tag::Tag,
};
use serde::Deserialize;
use time::OffsetDateTime;
use tokio::io::AsyncReadExt;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct LsEntry {
    name: String,
    #[serde(rename = "type")]
    r#type: Type,
    path: String,
    uid: Uid,
    gid: Gid,
    size: Option<FileSize>,
    mode: u32,
    permissions: String,
    #[serde(with = "time::serde::iso8601")]
    mtime: OffsetDateTime,
    #[serde(with = "time::serde::iso8601")]
    atime: OffsetDateTime,
    #[serde(with = "time::serde::iso8601")]
    ctime: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct SnapshotEntry {
    #[serde(with = "time::serde::iso8601")]
    time: OffsetDateTime,
    parent: Option<SnapshotId>,
    tree: TreeId,
    paths: Vec<backup::Path>,
    hostname: String,
    username: String,
    uid: Option<Uid>,
    gid: Option<Gid>,
    #[serde(default)]
    excludes: Vec<String>,
    #[serde(default)]
    tags: Vec<Tag>,
    id: SnapshotId,
    short_id: String,
}

impl SnapshotEntry {
    fn into_snapshot(self, repo_url: &repo::Url) -> Snapshot {
        let backup = self.tags.iter().find_map(|tag| tag.backup_name());
        Snapshot {
            repo_url: repo_url.clone(),
            backup,
            id: self.id,
            short_id: self.short_id,
            parent: self.parent,
            tree: self.tree,
            hostname: self.hostname,
            username: self.username,
            time: self.time,
            tags: self.tags,
        }
    }
}

pub async fn index_snapshots(
    restic: &Restic,
    db: &mut Database,
    repo: &RepoWithSecrets<'_>,
) -> eyre::Result<u64> {
    let mut process = restic.run(
        Some(repo),
        &["snapshots"],
        &Options {
            stdout: Output::Capture,
            json: true,
            ..Default::default()
        },
    )?;
    let mut buf = Vec::new();
    process
        .stdout()
        .as_mut()
        .expect("should be present based on params")
        .read_to_end(&mut buf)
        .await?;
    let snapshots: Vec<SnapshotEntry> = serde_json::from_slice(&buf)?;
    db.save_snapshots(
        &repo.repo,
        snapshots
            .into_iter()
            .map(|e| e.into_snapshot(&repo.repo.url)),
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    #[test]
    fn should_parse_dir_entry() {
        // language=JSON
        let json = r#"{
          "name": "a-directory",
          "type": "dir",
          "path": "/var/tmp/subdir/a-directory",
          "uid": 1000,
          "gid": 1000,
          "mode": 2147484157,
          "permissions": "drwxrwxr-x",
          "mtime": "2022-06-05T13:46:04.582083272+02:00",
          "atime": "2022-06-05T13:56:04.582083272+02:00",
          "ctime": "2022-06-05T13:16:04.582083272+02:00",
          "struct_type": "node"
        }"#;

        let entry: LsEntry = serde_json::from_str(json).unwrap();

        assert_eq!(
            entry,
            LsEntry {
                name: "a-directory".to_string(),
                r#type: Type::Dir,
                path: "/var/tmp/subdir/a-directory".to_string(),
                uid: Uid(1000),
                gid: Gid(1000),
                size: None,
                mode: 0o20000000775,
                permissions: "drwxrwxr-x".to_string(),
                mtime: datetime!(2022-06-05 13:46:04.582083272 +02:00),
                atime: datetime!(2022-06-05 13:56:04.582083272 +02:00),
                ctime: datetime!(2022-06-05 13:16:04.582083272 +02:00),
            }
        )
    }

    #[test]
    fn should_parse_file_entry() {
        // language=JSON
        let json = r#"{
          "name": "test.yml",
          "type": "file",
          "path": "/test.yml",
          "uid": 0,
          "gid": 0,
          "size": 1234,
          "mode": 384,
          "permissions": "-rw-------",
          "mtime": "2022-10-22T13:46:04.582083272+02:00",
          "atime": "2022-10-22T13:56:04.582083272+02:00",
          "ctime": "2022-10-22T13:16:04.582083272+02:00",
          "struct_type": "node"
        }"#;

        let entry: LsEntry = serde_json::from_str(json).unwrap();

        assert_eq!(
            entry,
            LsEntry {
                name: "test.yml".to_string(),
                r#type: Type::File,
                path: "/test.yml".to_string(),
                uid: Uid(0),
                gid: Gid(0),
                size: Some(FileSize(1234)),
                mode: 0o600,
                permissions: "-rw-------".to_string(),
                mtime: datetime!(2022-10-22 13:46:04.582083272 +02:00),
                atime: datetime!(2022-10-22 13:56:04.582083272 +02:00),
                ctime: datetime!(2022-10-22 13:16:04.582083272 +02:00),
            }
        )
    }

    #[test]
    fn should_parse_minimal_snapshot_entry() {
        // language=JSON
        let json = r#"
          {
            "time": "2020-08-03T23:05:57.5629523+02:00",
            "tree": "86fb8a32a6ac5c10fa2e21dbf140d8c40e5373dd891cc7926e067f125d6ad750",
            "paths": [
              "/root"
            ],
            "hostname": "testhost",
            "username": "root",
            "id": "3cc47d6ab8569b5bf8287d2b665b99f5279b2854a4c2a558676bae9e2741371d",
            "short_id": "3cc47d6a"
        }"#;

        let entry: SnapshotEntry = serde_json::from_str(json).unwrap();

        assert_eq!(
            entry,
            SnapshotEntry {
                time: datetime!(2020-08-03 23:05:57.5629523 +02:00),
                parent: None,
                tree: TreeId(
                    "86fb8a32a6ac5c10fa2e21dbf140d8c40e5373dd891cc7926e067f125d6ad750".to_string()
                ),
                paths: vec![backup::Path("/root".to_string())],
                hostname: "testhost".to_string(),
                username: "root".to_string(),
                uid: None,
                gid: None,
                excludes: vec![],
                tags: vec![],
                id: SnapshotId(
                    "3cc47d6ab8569b5bf8287d2b665b99f5279b2854a4c2a558676bae9e2741371d".to_string()
                ),
                short_id: "3cc47d6a".to_string()
            }
        )
    }

    #[test]
    fn should_parse_complete_snapshot_entry() {
        // language=JSON
        let json = r#"
          {
            "time": "2020-08-03T23:05:57.5629523+02:00",
            "tree": "86fb8a32a6ac5c10fa2e21dbf140d8c40e5373dd891cc7926e067f125d6ad750",
            "parent": "2e8ad31a949d004194b97031427161b5b9c5a846359629b4c0671e2bbb26e6c4",
            "paths": [
              "/"
            ],
            "hostname": "host",
            "username": "testuser",
            "uid": 1001,
            "gid": 1002,
            "excludes": [
              ".cache"
            ],
            "tags": [
              "tag1",
              "tag2.tag"
            ],
            "id": "3cc47d6ab8569b5bf8287d2b665b99f5279b2854a4c2a558676bae9e2741371d",
            "short_id": "3cc47d6a"
        }"#;

        let entry: SnapshotEntry = serde_json::from_str(json).unwrap();

        assert_eq!(
            entry,
            SnapshotEntry {
                time: datetime!(2020-08-03 23:05:57.5629523 +02:00),
                parent: Some(SnapshotId(
                    "2e8ad31a949d004194b97031427161b5b9c5a846359629b4c0671e2bbb26e6c4".to_string()
                )),
                tree: TreeId(
                    "86fb8a32a6ac5c10fa2e21dbf140d8c40e5373dd891cc7926e067f125d6ad750".to_string()
                ),
                paths: vec![backup::Path("/".to_string())],
                hostname: "host".to_string(),
                username: "testuser".to_string(),
                uid: Some(Uid(1001)),
                gid: Some(Gid(1002)),
                excludes: vec![".cache".to_string()],
                tags: vec![Tag("tag1".to_string()), Tag("tag2.tag".to_string())],
                id: SnapshotId(
                    "3cc47d6ab8569b5bf8287d2b665b99f5279b2854a4c2a558676bae9e2741371d".to_string()
                ),
                short_id: "3cc47d6a".to_string()
            }
        )
    }
}
