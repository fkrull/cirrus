use crate::config::repo;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Name(pub String);

#[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct Path(pub String);

#[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Exclude(pub String);

#[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
pub struct Trigger(pub schedule_dsl::Schedule);

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub struct Definition {
    pub repository: repo::Name,
    pub path: Path,
    #[serde(default)]
    pub excludes: Vec<Exclude>,
    #[serde(default, alias = "exclude_caches")]
    pub exclude_caches: bool,
    #[serde(default, alias = "exclude_larger_than")]
    pub exclude_larger_than: Option<String>,
    #[serde(default, alias = "extra_args")]
    pub extra_args: Vec<String>,
    #[serde(default, alias = "disable_triggers")]
    pub disable_triggers: bool,
    #[serde(default)]
    pub triggers: Vec<Trigger>,
}
