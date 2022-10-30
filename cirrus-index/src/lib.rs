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

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct Owner {
    pub uid: Uid,
    pub gid: Gid,
}

#[derive(Debug, PartialEq, Eq, Copy, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Mode(pub u32);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FileSize(pub u64);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SnapshotId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TreeHash(pub String);

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Type {
    Dir,
    File,
    Symlink,
    Fifo,
}

impl Type {
    fn to_i(self) -> u32 {
        match self {
            Type::Dir => 1,
            Type::File => 2,
            Type::Symlink => 3,
            Type::Fifo => 4,
        }
    }

    fn from_i(i: u32) -> Option<Type> {
        match i {
            1 => Some(Type::Dir),
            2 => Some(Type::File),
            3 => Some(Type::Symlink),
            4 => Some(Type::Fifo),
            _ => None,
        }
    }

    fn serialize_as_int<S: serde::Serializer>(v: &Type, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_u32(v.to_i())
    }

    fn deserialize_as_int<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Type, D::Error> {
        use serde::de::Error;
        let i = u32::deserialize(d)?;
        Type::from_i(i).ok_or_else(|| {
            Error::invalid_value(
                serde::de::Unexpected::Unsigned(i as u64),
                &"an integer in range [1..4]",
            )
        })
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub snapshot_id: SnapshotId,
    pub backup: Option<backup::Name>,
    pub parent: Option<SnapshotId>,
    pub tree_hash: TreeHash,
    pub hostname: String,
    pub username: String,
    #[serde(with = "time::serde::timestamp")]
    pub time: OffsetDateTime,
    #[serde(
        serialize_with = "Snapshot::serialize_tags",
        deserialize_with = "Snapshot::deserialize_tags"
    )]
    pub tags: Vec<Tag>,
}

impl Snapshot {
    pub fn short_id(&self) -> &str {
        &self.snapshot_id.0[0..8]
    }

    fn serialize_tags<S: serde::Serializer>(v: &[Tag], s: S) -> Result<S::Ok, S::Error> {
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
}

#[derive(Debug, PartialEq, Eq, Clone, Default, Serialize, Deserialize)]
#[serde(from = "String", into = "String")]
pub struct Parent(pub Option<String>);

impl Parent {
    pub fn path(&self, name: &str) -> String {
        match &self.0 {
            Some(parent) => format!("{parent}/{}", name),
            None => format!("/{}", name),
        }
    }
}

impl From<String> for Parent {
    fn from(s: String) -> Self {
        if s.is_empty() {
            Parent(None)
        } else {
            Parent(Some(s))
        }
    }
}

impl From<Parent> for String {
    fn from(p: Parent) -> Self {
        match p.0 {
            Some(s) => s,
            None => String::new(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Copy, Clone, Default, Serialize, Deserialize)]
#[serde(transparent)]
struct FileId(u64);

#[derive(Debug, PartialEq, Eq, Copy, Clone, Default, Serialize, Deserialize)]
#[serde(transparent)]
struct TreeId(u64);

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct File {
    #[serde(skip_serializing)]
    id: FileId,
    pub parent: Parent,
    pub name: String,
}

impl File {
    pub fn path(&self) -> String {
        self.parent.path(&self.name)
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct Version {
    file: FileId,
    tree: TreeId,
    #[serde(
        rename = "type",
        serialize_with = "Type::serialize_as_int",
        deserialize_with = "Type::deserialize_as_int"
    )]
    pub r#type: Type,
    #[serde(flatten)]
    pub owner: Owner,
    pub size: Option<FileSize>,
    pub mode: Mode,
    #[serde(with = "time::serde::timestamp")]
    pub mtime: OffsetDateTime,
    #[serde(with = "time::serde::timestamp")]
    pub ctime: OffsetDateTime,
}

#[cfg(test)]
mod tests {
    use super::*;

    mod parent {
        use super::*;
        use serde_json::Value;

        #[test]
        fn should_serialize_none() {
            let result = serde_json::to_value(Parent(None)).unwrap();

            assert_eq!(result, Value::from(""));
        }

        #[test]
        fn should_serialize_some() {
            let result = serde_json::to_value(Parent(Some("/parent".to_string()))).unwrap();

            assert_eq!(result, Value::from("/parent"));
        }

        #[test]
        fn should_deserialize_empty_string() {
            let result: Parent = serde_json::from_str(r#""""#).unwrap();

            assert_eq!(result, Parent(None));
        }

        #[test]
        fn should_deserialize_value() {
            let result: Parent = serde_json::from_str(r#""/tmp""#).unwrap();

            assert_eq!(result, Parent(Some("/tmp".to_string())));
        }
    }
}
