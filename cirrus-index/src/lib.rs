use cirrus_core::{config::backup, tag::Tag};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

mod db;
mod restic;
pub use db::Database;
pub use restic::{index_files, index_snapshots};

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
    Symlink,
    Fifo,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SnapshotId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TreeId(pub String);

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub snapshot_id: SnapshotId,
    pub backup: Option<backup::Name>,
    pub short_id: String,
    pub parent: Option<SnapshotId>,
    pub tree_id: TreeId,
    pub hostname: String,
    pub username: String,
    #[serde(with = "time::serde::iso8601")]
    pub time: OffsetDateTime,
    #[serde(
        serialize_with = "serialize_tags",
        deserialize_with = "deserialize_tags"
    )]
    pub tags: Vec<Tag>,
}

fn serialize_tags<S: serde::Serializer>(v: &Vec<Tag>, s: S) -> Result<S::Ok, S::Error> {
    use itertools::Itertools;
    let comma_separated = v.iter().map(|s| &s.0).join(",");
    s.serialize_str(&comma_separated)
}

fn deserialize_tags<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<Tag>, D::Error> {
    let comma_separated = String::deserialize(d)?;
    let split = comma_separated
        .split(',')
        .map(|s| Tag(s.to_string()))
        .collect();
    Ok(split)
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct Permissions {
    pub mode: u32,
    pub permissions_string: String,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct Owner {
    pub uid: Uid,
    pub gid: Gid,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct File {
    #[serde(skip_serializing)]
    id: u64,
    pub path: String,
    pub parent: Option<String>,
    pub name: String,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct Version {
    file: u64,
    pub tree_id: TreeId,
    #[serde(rename = "type")]
    pub r#type: Type,
    #[serde(flatten)]
    pub owner: Owner,
    pub size: Option<FileSize>,
    #[serde(flatten)]
    pub permissions: Permissions,
    #[serde(with = "time::serde::timestamp")]
    pub mtime: OffsetDateTime,
    #[serde(with = "time::serde::timestamp")]
    pub atime: OffsetDateTime,
    #[serde(with = "time::serde::timestamp")]
    pub ctime: OffsetDateTime,
}
