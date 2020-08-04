use crate::model::repo;
use crate::model::repo::{Secret, SecretName};
use anyhow::{anyhow, Context};
use std::collections::HashMap;

#[cfg(not(feature = "os-keyring"))]
mod keyring_disabled;
#[cfg(feature = "os-keyring")]
mod os_keyring;

#[cfg(not(feature = "os-keyring"))]
use keyring_disabled::*;
#[cfg(feature = "os-keyring")]
use os_keyring::*;

pub struct SecretValue(pub String);

impl SecretValue {
    pub fn new(value: impl Into<String>) -> Self {
        SecretValue(value.into())
    }
}

pub struct RepoSecrets {
    pub repo_password: SecretValue,
    pub secrets: HashMap<SecretName, SecretValue>,
}

#[derive(Debug)]
pub struct Secrets;

impl Secrets {
    pub fn get_secret(&self, secret: &Secret) -> anyhow::Result<SecretValue> {
        match secret {
            Secret::FromEnvVar { env_var } => {
                let value = std::env::var(env_var)
                    .context(format!("environment variable '{}' not set", env_var))?;
                Ok(SecretValue(value))
            }
            Secret::FromOsKeyring { keyring } => get_secret(keyring),
            Secret::FromToml { toml, key } => {
                let secrets_file = std::fs::read_to_string(toml)
                    .context(format!("failed to read secrets file '{}'", toml))?;
                let secrets: HashMap<&str, &str> = toml::from_str(&secrets_file)
                    .context(format!("failed to parse secrets file '{}'", toml))?;
                secrets
                    .get(key.as_str())
                    .map(|s| s.to_owned())
                    .ok_or_else(|| anyhow!("key '{}' not found in secrets file '{}'", key, toml))
                    .map(SecretValue::new)
            }
        }
    }

    pub fn set_secret(&self, secret: &Secret, value: SecretValue) -> anyhow::Result<()> {
        match secret {
            Secret::FromOsKeyring { keyring } => set_secret(keyring, value),
            _ => Err(anyhow!(
                "{} secret must be configured externally",
                secret.label()
            )),
        }
    }

    pub fn get_secrets(&self, repo: &repo::Definition) -> anyhow::Result<RepoSecrets> {
        let password = self.get_secret(&repo.password)?;
        let secrets = repo
            .secrets
            .iter()
            .map(|(name, secret)| {
                let value = self.get_secret(secret)?;
                Ok((name.clone(), value))
            })
            .collect::<anyhow::Result<HashMap<_, _>>>()?;
        Ok(RepoSecrets {
            repo_password: password,
            secrets,
        })
    }
}
