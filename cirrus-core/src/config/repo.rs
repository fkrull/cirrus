use serde::{Deserialize, Serialize};
use std::{collections::HashMap, time::Duration};

#[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Secret {
    FromEnvVar {
        #[serde(rename = "env-var", alias = "env_var")]
        env_var: String,
    },
    FromOsKeyring {
        keyring: String,
    },
    FromToml {
        toml: String,
        key: String,
    },
}

impl Default for Secret {
    fn default() -> Self {
        Secret::FromEnvVar {
            env_var: Default::default(),
        }
    }
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

#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Name(pub String);

#[derive(Debug, Default, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Url(pub String);

#[derive(Debug, Default, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SecretName(pub String);

#[derive(Debug, Default, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct Definition {
    pub url: Url,
    #[serde(alias = "parallel-jobs")]
    pub parallel_jobs: Option<u32>,
    #[serde(default, with = "humantime_serde", alias = "build-index")]
    pub build_index: Option<Duration>,
    pub password: Secret,
    #[serde(default)]
    pub secrets: HashMap<SecretName, Secret>,
}
