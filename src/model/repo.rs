use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Secret {
    FromEnvVar { env_var: String },
    FromOsKeyring { keyring: String },
    FromToml { toml: String, key: String },
    InlinePlain { inline: String },
}

impl Secret {
    pub fn label(&self) -> &str {
        match self {
            Secret::FromEnvVar { .. } => "environment variable",
            Secret::FromOsKeyring { .. } => "OS keyring",
            Secret::FromToml { .. } => "TOML value",
            Secret::InlinePlain { .. } => "inline",
        }
    }
}

impl std::fmt::Debug for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Secret::FromEnvVar { env_var } => f
                .debug_struct("Secret::FromEnvVar")
                .field("env_var", env_var)
                .finish(),
            Secret::FromOsKeyring { keyring } => f
                .debug_struct("Secret::FromOsKeyring")
                .field("keyring", keyring)
                .finish(),
            Secret::FromToml { toml, key } => f
                .debug_struct("Secret::FromToml")
                .field("toml", toml)
                .field("key", key)
                .finish(),
            Secret::InlinePlain { .. } => f
                .debug_struct("Secret::InlinePlain")
                .field("inline", &"***")
                .finish(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Name(pub String);

#[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Url(pub String);

#[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SecretName(pub String);

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct Definition {
    pub url: Url,
    pub password: Secret,
    #[serde(default)]
    pub secrets: HashMap<SecretName, Secret>,
}
