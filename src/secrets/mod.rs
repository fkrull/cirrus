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

pub struct SecretValue(pub(crate) String);

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
    fn get_secret(&self, secret: &Secret) -> anyhow::Result<SecretValue> {
        match secret {
            Secret::FromEnvVar { env_var } => {
                let value = std::env::var(env_var)
                    .context(format!("environment variable '{}' not set", env_var))?;
                Ok(SecretValue(value))
            }
            Secret::FromOsKeyring { keyring } => get_secret(keyring),
            Secret::InlinePlain { inline } => {
                // TODO: remove this maybe?
                Ok(SecretValue(inline.clone()))
            }
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

    pub fn set_secret(&self, secret: &Secret, value: SecretValue) -> anyhow::Result<()> {
        match secret {
            Secret::FromOsKeyring { keyring } => set_secret(keyring, value),
            _ => Err(anyhow!(
                "{} secret must be configured externally",
                secret.label()
            )),
        }
    }
}
