use crate::{
    Database, File, FileSize, Gid, Mode, Owner, Parent, Snapshot, SnapshotId, TreeHash, Type, Uid,
    Version,
};
use cirrus_core::{
    config::backup,
    restic::{Options, Output, Restic},
    secrets::RepoWithSecrets,
    tag::Tag,
};
use futures::{StreamExt, TryStreamExt};
use serde::Deserialize;
use time::OffsetDateTime;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct SnapshotJson {
    #[serde(with = "time::serde::iso8601")]
    time: OffsetDateTime,
    parent: Option<SnapshotId>,
    tree: TreeHash,
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

impl SnapshotJson {
    fn into_snapshot(self) -> Snapshot {
        let backup = self.tags.iter().find_map(|tag| tag.backup_name());
        Snapshot {
            snapshot_id: self.id,
            backup,
            parent: self.parent,
            tree_hash: self.tree,
            hostname: self.hostname,
            username: self.username,
            time: self.time,
            tags: self.tags,
        }
    }
}

fn get_parent(path: &str, name: &str) -> Parent {
    Parent(
        path.strip_suffix(name)
            .and_then(|s| s.strip_suffix('/'))
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string()),
    )
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct NodeJson {
    name: String,
    r#type: Type,
    path: String,
    uid: Uid,
    gid: Gid,
    size: Option<FileSize>,
    mode: Mode,
    permissions: String,
    #[serde(with = "time::serde::iso8601")]
    mtime: OffsetDateTime,
    #[serde(with = "time::serde::iso8601")]
    atime: OffsetDateTime,
    #[serde(with = "time::serde::iso8601")]
    ctime: OffsetDateTime,
}

impl NodeJson {
    fn into_file_and_version(self) -> (File, Version) {
        let parent = get_parent(&self.path, &self.name);
        let file = File {
            parent,
            name: self.name,
            r#type: self.r#type,
        };
        let version = Version {
            owner: Owner {
                uid: self.uid,
                gid: self.gid,
            },
            size: self.size,
            mode: self.mode,
            mtime: self.mtime,
            ctime: self.ctime,
        };
        (file, version)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
#[serde(tag = "struct_type")]
enum LsJson {
    Snapshot(SnapshotJson),
    Node(NodeJson),
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
    let snapshots: Vec<SnapshotJson> = serde_json::from_slice(&buf)?;
    let ret = db
        .save_snapshots(snapshots.into_iter().map(|e| e.into_snapshot()))
        .await?;
    process.check_wait().await?;
    Ok(ret)
}

pub async fn index_files(
    restic: &Restic,
    db: &mut Database,
    repo: &RepoWithSecrets<'_>,
    snapshot: &Snapshot,
) -> eyre::Result<u64> {
    let mut process = restic.run(
        Some(repo),
        &["ls", &snapshot.snapshot_id.0],
        &Options {
            stdout: Output::Capture,
            json: true,
            ..Default::default()
        },
    )?;

    let files = tokio_stream::wrappers::LinesStream::new(
        BufReader::new(
            process
                .stdout()
                .as_mut()
                .expect("should be present based on params"),
        )
        .lines(),
    )
    .map(|line| Ok::<_, eyre::Report>(serde_json::from_str::<LsJson>(&line?)?))
    .try_filter_map(|json| async move {
        match json {
            LsJson::Snapshot(_) => Ok(None),
            LsJson::Node(node) => Ok(Some(node.into_file_and_version())),
        }
    });

    db.save_files(snapshot, files).await
}

#[cfg(test)]
mod tests {
    use super::*;

    mod json {
        use super::*;
        use time::macros::datetime;

        #[test]
        fn should_parse_dir_node() {
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

            let result: LsJson = serde_json::from_str(json).unwrap();

            assert_eq!(
                result,
                LsJson::Node(NodeJson {
                    name: "a-directory".to_string(),
                    r#type: Type::Dir,
                    path: "/var/tmp/subdir/a-directory".to_string(),
                    uid: Uid(1000),
                    gid: Gid(1000),
                    size: None,
                    mode: Mode(0o20000000775),
                    permissions: "drwxrwxr-x".to_string(),
                    mtime: datetime!(2022-06-05 13:46:04.582083272 +02:00),
                    atime: datetime!(2022-06-05 13:56:04.582083272 +02:00),
                    ctime: datetime!(2022-06-05 13:16:04.582083272 +02:00),
                })
            );
        }

        #[test]
        fn should_parse_file_node() {
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

            let result: LsJson = serde_json::from_str(json).unwrap();

            assert_eq!(
                result,
                LsJson::Node(NodeJson {
                    name: "test.yml".to_string(),
                    r#type: Type::File,
                    path: "/test.yml".to_string(),
                    uid: Uid(0),
                    gid: Gid(0),
                    size: Some(FileSize(1234)),
                    mode: Mode(0o600),
                    permissions: "-rw-------".to_string(),
                    mtime: datetime!(2022-10-22 13:46:04.582083272 +02:00),
                    atime: datetime!(2022-10-22 13:56:04.582083272 +02:00),
                    ctime: datetime!(2022-10-22 13:16:04.582083272 +02:00),
                })
            );
        }

        #[test]
        fn should_parse_symlink_node() {
            // language=JSON
            let json = r#"{
          "name": "testlink",
          "type": "symlink",
          "path": "/tmp/testlink",
          "uid": 0,
          "gid": 0,
          "mode": 384,
          "permissions": "-rw-------",
          "mtime": "2022-10-22T13:46:04.582083272+02:00",
          "atime": "2022-10-22T13:56:04.582083272+02:00",
          "ctime": "2022-10-22T13:16:04.582083272+02:00",
          "struct_type": "node"
        }"#;

            let result: LsJson = serde_json::from_str(json).unwrap();

            assert_eq!(
                result,
                LsJson::Node(NodeJson {
                    name: "testlink".to_string(),
                    r#type: Type::Symlink,
                    path: "/tmp/testlink".to_string(),
                    uid: Uid(0),
                    gid: Gid(0),
                    size: None,
                    mode: Mode(0o600),
                    permissions: "-rw-------".to_string(),
                    mtime: datetime!(2022-10-22 13:46:04.582083272 +02:00),
                    atime: datetime!(2022-10-22 13:56:04.582083272 +02:00),
                    ctime: datetime!(2022-10-22 13:16:04.582083272 +02:00),
                })
            );
        }

        #[test]
        fn should_parse_snapshot_ls_json() {
            // language=JSON
            let json = r#"{
          "time": "2022-10-28T18:30:26.123+00:00",
          "parent": "par",
          "tree": "tree",
          "paths": [
            "C:\\"
          ],
          "hostname": "test",
          "username": "testuser",
          "tags": [
            "testtag"
          ],
          "id": "id",
          "short_id": "short_id",
          "struct_type": "snapshot"
        }"#;

            let result: LsJson = serde_json::from_str(json).unwrap();

            assert_eq!(
                result,
                LsJson::Snapshot(SnapshotJson {
                    time: datetime!(2022-10-28 18:30:26.123 +00:00),
                    parent: Some(SnapshotId("par".to_string())),
                    tree: TreeHash("tree".to_string()),
                    paths: vec![backup::Path("C:\\".to_string())],
                    hostname: "test".to_string(),
                    username: "testuser".to_string(),
                    uid: None,
                    gid: None,
                    excludes: vec![],
                    tags: vec![Tag("testtag".to_string())],
                    id: SnapshotId("id".to_string()),
                    short_id: "short_id".to_string()
                })
            );
        }

        #[test]
        fn should_parse_fifo_node() {
            // language=JSON
            let json = r#"{
          "name": "pipe",
          "type": "fifo",
          "path": "/tmp/pipe",
          "uid": 0,
          "gid": 0,
          "mode": 384,
          "permissions": "-rw-------",
          "mtime": "2022-10-22T13:46:04.582083272+02:00",
          "atime": "2022-10-22T13:56:04.582083272+02:00",
          "ctime": "2022-10-22T13:16:04.582083272+02:00",
          "struct_type": "node"
        }"#;

            let result: LsJson = serde_json::from_str(json).unwrap();

            assert_eq!(
                result,
                LsJson::Node(NodeJson {
                    name: "pipe".to_string(),
                    r#type: Type::Fifo,
                    path: "/tmp/pipe".to_string(),
                    uid: Uid(0),
                    gid: Gid(0),
                    size: None,
                    mode: Mode(0o600),
                    permissions: "-rw-------".to_string(),
                    mtime: datetime!(2022-10-22 13:46:04.582083272 +02:00),
                    atime: datetime!(2022-10-22 13:56:04.582083272 +02:00),
                    ctime: datetime!(2022-10-22 13:16:04.582083272 +02:00),
                })
            );
        }

        #[test]
        fn should_parse_minimal_snapshot_json() {
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

            let result: SnapshotJson = serde_json::from_str(json).unwrap();

            assert_eq!(
                result,
                SnapshotJson {
                    time: datetime!(2020-08-03 23:05:57.5629523 +02:00),
                    parent: None,
                    tree: TreeHash(
                        "86fb8a32a6ac5c10fa2e21dbf140d8c40e5373dd891cc7926e067f125d6ad750"
                            .to_string()
                    ),
                    paths: vec![backup::Path("/root".to_string())],
                    hostname: "testhost".to_string(),
                    username: "root".to_string(),
                    uid: None,
                    gid: None,
                    excludes: vec![],
                    tags: vec![],
                    id: SnapshotId(
                        "3cc47d6ab8569b5bf8287d2b665b99f5279b2854a4c2a558676bae9e2741371d"
                            .to_string()
                    ),
                    short_id: "3cc47d6a".to_string()
                }
            )
        }

        #[test]
        fn should_parse_complete_snapshot_json() {
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

            let result: SnapshotJson = serde_json::from_str(json).unwrap();

            assert_eq!(
                result,
                SnapshotJson {
                    time: datetime!(2020-08-03 23:05:57.5629523 +02:00),
                    parent: Some(SnapshotId(
                        "2e8ad31a949d004194b97031427161b5b9c5a846359629b4c0671e2bbb26e6c4"
                            .to_string()
                    )),
                    tree: TreeHash(
                        "86fb8a32a6ac5c10fa2e21dbf140d8c40e5373dd891cc7926e067f125d6ad750"
                            .to_string()
                    ),
                    paths: vec![backup::Path("/".to_string())],
                    hostname: "host".to_string(),
                    username: "testuser".to_string(),
                    uid: Some(Uid(1001)),
                    gid: Some(Gid(1002)),
                    excludes: vec![".cache".to_string()],
                    tags: vec![Tag("tag1".to_string()), Tag("tag2.tag".to_string())],
                    id: SnapshotId(
                        "3cc47d6ab8569b5bf8287d2b665b99f5279b2854a4c2a558676bae9e2741371d"
                            .to_string()
                    ),
                    short_id: "3cc47d6a".to_string()
                }
            )
        }
    }

    mod get_parent {
        use super::*;

        #[test]
        fn should_get_parent() {
            let path = "/home/user/name";
            let name = "name";

            let result = get_parent(path, name);

            assert_eq!(result, Parent(Some("/home/user".to_string())));
        }

        #[test]
        fn should_not_get_parent_for_toplevel_dir() {
            let path = "/C";
            let name = "C";

            let result = get_parent(path, name);

            assert_eq!(result, Parent(None));
        }

        #[test]
        fn should_not_get_parent_with_non_matching_name() {
            let path = "/home/user/name";
            let name = "test";

            let result = get_parent(path, name);

            assert_eq!(result, Parent(None));
        }

        #[test]
        fn should_not_get_parent_with_non_matching_name_and_prefix() {
            let path = "/home/user/namename";
            let name = "name";

            let result = get_parent(path, name);

            assert_eq!(result, Parent(None));
        }
    }
}
