use cirrus_core::config::repo;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

mod migrations;
mod restic_json;

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
    pub repo: repo::Name,
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
