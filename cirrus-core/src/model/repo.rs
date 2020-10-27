use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Secret {
    FromEnvVar { env_var: String },
    FromOsKeyring { keyring: String },
    FromToml { toml: String, key: String },
}

impl Secret {
    pub fn label(&self) -> &str {
        match self {
            Secret::FromEnvVar { .. } => "environment variable",
            Secret::FromOsKeyring { .. } => "OS keyring",
            Secret::FromToml { .. } => "TOML value",
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct Name(pub String);

#[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Url(pub String);

#[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SecretName(pub String);

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Definition {
    pub url: Url,
    pub password: Secret,
    #[serde(default)]
    pub secrets: HashMap<SecretName, Secret>,
}
