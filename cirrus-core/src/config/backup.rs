use crate::config::repo;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Name(pub String);

#[derive(Debug, Default, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Path(pub String);

#[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Exclude(pub String);

#[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
pub struct Trigger(pub schedule_dsl::Schedule);

#[derive(Debug, Default, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct Definition {
    pub repository: repo::Name,
    pub path: Path,
    #[serde(default)]
    pub excludes: Vec<Exclude>,
    #[serde(default, alias = "exclude-caches")]
    pub exclude_caches: bool,
    #[serde(default, alias = "exclude-larger-than")]
    pub exclude_larger_than: Option<String>,
    #[serde(default, alias = "ignore-unreadable-source-files")]
    pub ignore_unreadable_source_files: bool,
    #[serde(default, alias = "extra-args")]
    pub extra_args: Vec<String>,
    #[serde(default, alias = "disable-triggers")]
    pub disable_triggers: bool,
    #[serde(default)]
    pub triggers: Vec<Trigger>,
}
