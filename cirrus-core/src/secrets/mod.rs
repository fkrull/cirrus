use crate::config::repo::{self, Secret, SecretName};
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
            Secret::FromEnvVar { env_var } => {
                std::env::set_var(env_var, &value.0);
                Ok(())
            }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    struct EnvGuard(String);

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            std::env::remove_var(&self.0);
        }
    }

    fn set_env(key: &str, value: &str) -> EnvGuard {
        std::env::set_var(key, value);
        EnvGuard(key.to_owned())
    }

    mod env_var {
        use super::*;

        #[test]
        fn should_get_secret() {
            let secret = Secret::FromEnvVar {
                env_var: "TEST_SECRET1".to_owned(),
            };
            let _guard = set_env("TEST_SECRET1", "test-secret-value");

            let value = Secrets.get_secret(&secret).unwrap();

            assert_eq!(&value.0, "test-secret-value");
        }

        #[test]
        fn should_not_get_secret_if_missing_env_var() {
            let secret = Secret::FromEnvVar {
                env_var: "TEST_SECRET2".to_owned(),
            };
            std::env::remove_var("TEST_SECRET2");

            let result = Secrets.get_secret(&secret);

            assert!(result.is_err());
        }

        #[test]
        fn should_set_secret() {
            let _guard = set_env("TEST_SECRET", "");
            let secret = Secret::FromEnvVar {
                env_var: "TEST_SECRET".to_owned(),
            };

            Secrets
                .set_secret(&secret, SecretValue("secret-value".to_owned()))
                .unwrap();

            assert_eq!(&std::env::var("TEST_SECRET").unwrap(), "secret-value");
        }
    }

    mod toml {
        use super::*;

        #[test]
        fn should_get_secret() {
            let mut tmp = tempfile::NamedTempFile::new().unwrap();
            tmp.write_all(br#"secret = "secret-value""#).unwrap();
            let secret = Secret::FromToml {
                toml: tmp.path().to_str().unwrap().to_owned(),
                key: "secret".to_owned(),
            };

            let value = Secrets.get_secret(&secret).unwrap();

            assert_eq!(&value.0, "secret-value");
        }

        #[test]
        fn should_not_get_secret_if_missing_key() {
            let mut tmp = tempfile::NamedTempFile::new().unwrap();
            tmp.write_all(br#"secret1 = "secret-value""#).unwrap();
            let secret = Secret::FromToml {
                toml: tmp.path().to_str().unwrap().to_owned(),
                key: "secret2".to_owned(),
            };

            let result = Secrets.get_secret(&secret);

            assert!(result.is_err());
        }

        #[test]
        fn should_not_get_secret_if_missing_file() {
            let secret = Secret::FromToml {
                toml: "/tmp/nopenopenope".to_owned(),
                key: "secret".to_owned(),
            };

            let result = Secrets.get_secret(&secret);

            assert!(result.is_err());
        }

        #[test]
        fn should_not_get_secret_if_invalid_toml_file() {
            let mut tmp = tempfile::NamedTempFile::new().unwrap();
            tmp.write_all(b"secret = haha").unwrap();
            let secret = Secret::FromToml {
                toml: tmp.path().to_str().unwrap().to_owned(),
                key: "secret".to_owned(),
            };

            let result = Secrets.get_secret(&secret);

            assert!(result.is_err());
        }

        #[test]
        fn should_not_set_secret() {
            let secret = Secret::FromToml {
                toml: "/tmp/nope.toml".to_string(),
                key: "key".to_string(),
            };

            let result = Secrets.set_secret(&secret, SecretValue("nope".to_owned()));

            assert!(result.is_err());
        }
    }

    #[test]
    fn should_load_all_secrets_for_repo() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        tmp.write_all(b"password = \"repo-pwd\"\ntoml-secret2 = \"toml-secret-value\"")
            .unwrap();
        let toml_path = tmp.path().to_str().unwrap().to_owned();
        let repo = repo::Definition {
            url: repo::Url("local:/srv/repo".to_owned()),
            parallel_jobs: None,
            password: Secret::FromToml {
                toml: toml_path.clone(),
                key: "password".to_owned(),
            },
            secrets: maplit::hashmap! {
                SecretName("secret1".to_owned()) => Secret::FromEnvVar {
                    env_var: "ENV_SECRET".to_owned()
                },
                SecretName("secret2".to_owned()) => Secret::FromToml {
                    toml: toml_path.clone(),
                    key: "toml-secret2".to_owned()
                },
            },
        };
        let _guard = set_env("ENV_SECRET", "env-secret-value");

        let repo_with_secrets = Secrets.get_secrets(&repo).unwrap();

        assert_eq!(repo_with_secrets.repo, &repo);
        assert_eq!(&repo_with_secrets.repo_password.0, "repo-pwd");
        let secrets = repo_with_secrets
            .secrets
            .iter()
            .map(|(key, value)| (key.0.as_str(), value.0.as_str()))
            .collect::<HashMap<_, _>>();
        assert_eq!(
            secrets,
            maplit::hashmap! {
                "secret1" => "env-secret-value",
                "secret2" => "toml-secret-value",
            }
        );
    }
}
