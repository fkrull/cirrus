use crate::model::repo;
use crate::model::repo::{Secret, SecretName};
use anyhow::anyhow;
use anyhow::Context;
use keyring::Keyring;
use std::collections::HashMap;

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
    const KEYRING_SERVICE: &'static str = "cirrus-backup";

    fn get_secret(&self, secret: &Secret) -> anyhow::Result<SecretValue> {
        match secret {
            Secret::FromEnvVar { env_var } => {
                let value = std::env::var(env_var)
                    .context(format!("environment variable '{}' not set", env_var))?;
                Ok(SecretValue(value))
            }
            Secret::FromOsKeyring { keyring } => {
                let value = Keyring::new(Self::KEYRING_SERVICE, keyring)
                    .get_password()
                    .context(format!("no stored password for key '{}'", keyring))?;
                Ok(SecretValue(value))
            }
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
            Secret::FromOsKeyring { keyring } => Keyring::new(Self::KEYRING_SERVICE, keyring)
                .set_password(&value.0)
                .context(format!("failed to set value for key '{}'", keyring)),
            _ => Err(anyhow!(
                "{} secret must be configured externally",
                secret.label()
            )),
        }
    }
}
