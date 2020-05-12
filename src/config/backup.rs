use super::repo;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Name(pub String);

#[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Path(pub String);

#[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Exclude(pub String);

#[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Trigger {
    Cron { cron: String },
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
pub struct Definition {
    pub repository: repo::Name,
    pub path: Path,
    #[serde(default)]
    pub excludes: Vec<Exclude>,
    #[serde(default)]
    pub extra_args: Vec<String>,
    pub triggers: Vec<Trigger>,
}
