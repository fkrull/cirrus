use crate::{
    model::repo,
    model::repo::{Secret, SecretName},
};
use eyre::{eyre, WrapErr};
use std::collections::HashMap;

#[cfg(not(feature = "keyring"))]
mod keyring_disabled;
#[cfg(feature = "keyring")]
mod os_keyring;

#[cfg(not(feature = "keyring"))]
use keyring_disabled::*;
#[cfg(feature = "keyring")]
use os_keyring::*;

pub struct SecretValue(pub String);

impl SecretValue {
    pub fn new(value: impl Into<String>) -> Self {
        SecretValue(value.into())
    }
}

pub struct RepoWithSecrets<'a> {
    pub repo: &'a repo::Definition,
    pub repo_password: SecretValue,
    pub secrets: HashMap<SecretName, SecretValue>,
}

#[derive(Debug)]
pub struct Secrets;

impl Secrets {
    pub fn get_secret(&self, secret: &Secret) -> eyre::Result<SecretValue> {
        match secret {
            Secret::FromEnvVar { env_var } => {
                let value = std::env::var(env_var)
                    .wrap_err_with(|| format!("environment variable '{}' not set", env_var))?;
                Ok(SecretValue(value))
            }
            Secret::FromOsKeyring { keyring } => get_secret(keyring),
            Secret::FromToml { toml, key } => {
                let secrets_file = std::fs::read_to_string(toml)
                    .wrap_err_with(|| format!("failed to read secrets file '{}'", toml))?;
                let secrets: HashMap<&str, &str> = toml::from_str(&secrets_file)
                    .wrap_err_with(|| format!("failed to parse secrets file '{}'", toml))?;
                secrets
                    .get(key.as_str())
                    .map(|s| s.to_owned())
                    .ok_or_else(|| eyre!("key '{}' not found in secrets file '{}'", key, toml))
                    .map(SecretValue::new)
            }
        }
    }

    pub fn set_secret(&self, secret: &Secret, value: SecretValue) -> eyre::Result<()> {
        match secret {
            Secret::FromOsKeyring { keyring } => set_secret(keyring, value),
            _ => Err(eyre!(
                "{} secret must be configured externally",
                secret.label()
            )),
        }
    }

    pub fn get_secrets<'a>(&self, repo: &'a repo::Definition) -> eyre::Result<RepoWithSecrets<'a>> {
        let repo_password = self.get_secret(&repo.password)?;
        let secrets = repo
            .secrets
            .iter()
            .map(|(name, secret)| {
                let value = self.get_secret(secret)?;
                Ok((name.clone(), value))
            })
            .collect::<eyre::Result<HashMap<_, _>>>()?;
        Ok(RepoWithSecrets {
            repo,
            repo_password,
            secrets,
        })
    }
}
