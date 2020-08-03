use crate::model::repo;
use crate::model::repo::{Secret, SecretName};
use anyhow::Context;
use std::collections::HashMap;

#[derive(Debug)]
pub struct SecretValue(pub(crate) String);

#[derive(Debug)]
pub struct RepoSecrets {
    pub repo_password: SecretValue,
    pub secrets: HashMap<SecretName, SecretValue>,
}

fn get_secret(secret: &Secret) -> anyhow::Result<SecretValue> {
    match secret {
        Secret::FromEnvVar { env_var } => {
            let value = std::env::var(env_var)
                .context(format!("environment variable '{}' not set", env_var))?;
            Ok(SecretValue(value))
        }
        Secret::InlinePlain { plain } => {
            // TODO: remove this maybe?
            Ok(SecretValue(plain.clone()))
        }
    }
}

pub fn get_secrets(repo: &repo::Definition) -> anyhow::Result<RepoSecrets> {
    let password = get_secret(&repo.password)?;
    let secrets = repo
        .secrets
        .iter()
        .map(|(name, secret)| {
            let value = get_secret(secret)?;
            Ok((name.clone(), value))
        })
        .collect::<anyhow::Result<HashMap<_, _>>>()?;
    Ok(RepoSecrets {
        repo_password: password,
        secrets,
    })
}
