use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};
use thiserror::Error;

pub mod backup;
pub mod repo;

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct Repositories(pub HashMap<repo::Name, repo::Definition>);

impl Repositories {
    pub fn get(&self, name: &repo::Name) -> Option<&repo::Definition> {
        self.0.get(name)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&repo::Name, &repo::Definition)> {
        self.0.iter()
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct Backups(pub HashMap<backup::Name, backup::Definition>);

impl Backups {
    pub fn get(&self, name: &backup::Name) -> Option<&backup::Definition> {
        self.0.get(name)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&backup::Name, &backup::Definition)> {
        self.0.iter()
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    pub repositories: Repositories,
    pub backups: Backups,

    /// path of the configuration file, if the configuration was loaded from a file
    #[serde(skip)]
    pub source: Option<PathBuf>,
}

#[derive(Debug, Error)]
pub enum ConfigLoadError {
    #[error("invalid configuration string")]
    InvalidConfigString(String, #[source] eyre::Report),
    #[error("invalid configuration file {}", .0.display())]
    InvalidConfigFile(PathBuf, #[source] eyre::Report),
    #[error("i/o error reading configuration file {}", .0.display())]
    IoError(PathBuf, std::io::Error),
}

#[derive(Debug, Error)]
#[error("unknown repository '{}'", (self.0).0)]
pub struct UnknownRepository(repo::Name);

#[derive(Debug, Error)]
#[error("unknown backup '{}'", (self.0).0)]
pub struct UnknownBackup(backup::Name);

impl Config {
    pub fn from_str(s: &str) -> Result<Config, ConfigLoadError> {
        toml::from_str(s).map_err(|e| ConfigLoadError::InvalidConfigString(s.to_owned(), e.into()))
    }

    pub async fn from_file(p: &Path) -> Result<Config, ConfigLoadError> {
        let config_string = tokio::fs::read_to_string(p)
            .await
            .map_err(|e| ConfigLoadError::IoError(p.to_owned(), e))?;
        let mut config: Config = toml::from_str(&config_string)
            .map_err(|e| ConfigLoadError::InvalidConfigFile(p.to_owned(), e.into()))?;
        config.source = Some(p.to_owned());
        Ok(config)
    }

    pub fn repository(&self, name: &repo::Name) -> Result<&repo::Definition, UnknownRepository> {
        self.repositories
            .0
            .get(name)
            .ok_or_else(|| UnknownRepository(name.clone()))
    }

    pub fn backup(&self, name: &backup::Name) -> Result<&backup::Definition, UnknownBackup> {
        self.backups
            .0
            .get(name)
            .ok_or_else(|| UnknownBackup(name.clone()))
    }

    pub fn repository_for_backup(
        &self,
        backup: &backup::Definition,
    ) -> Result<&repo::Definition, UnknownRepository> {
        self.repository(&backup.repository)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trigger;
    use maplit::hashmap;

    #[test]
    fn should_parse_complex_config() -> eyre::Result<()> {
        let input: toml::Value = toml::from_str(
            //language=TOML
            r#"
            [repositories.local]
            url = "/srv/restic-repo"
            password = { env-var = "LOCAL_PASSWORD" }

            [repositories.sftp]
            url = "sftp:user@host:repo/path"
            password = { env-var = "SSH_PASSWORD" }
            
            [repositories.sftp.secrets.UNUSED_SECRET]
            env-var = "SECRET_ENV"

            [backups.home]
            repository = "local"
            path = "/home/user"
            excludes = [
                "/.local/share/Trash",
                "/.cache"
            ]
            exclude-caches = true
            exclude-larger-than = "1G"
            extra-args = ["--one-file-system"]

            # look I don't remember cron syntax
            [[backups.home.triggers]]
            cron = "2 * *"
            timezone = "Europe/Berlin"
            [[backups.home.triggers]]
            cron = "1 * *"

            [backups.srv]
            repository = "sftp"
            path = "/srv"
            disable-triggers = true
            triggers = []
            "#,
        )?;

        let config: Config = input.try_into()?;

        assert_eq!(
            config,
            Config {
                repositories: Repositories(hashmap! {
                    repo::Name("local".to_string()) => repo::Definition {
                        url: repo::Url("/srv/restic-repo".to_string()),
                        password: repo::Secret::FromEnvVar { env_var: "LOCAL_PASSWORD".to_string() },
                        secrets: HashMap::new(),
                    },
                    repo::Name("sftp".to_string()) => repo::Definition {
                        url: repo::Url("sftp:user@host:repo/path".to_string()),
                        password: repo::Secret::FromEnvVar { env_var: "SSH_PASSWORD".to_string() },
                        secrets: hashmap! {
                            repo::SecretName("UNUSED_SECRET".to_string()) => repo::Secret::FromEnvVar {
                                env_var: "SECRET_ENV".to_string()
                            }
                        }
                    },
                }),
                backups: Backups(hashmap! {
                    backup::Name("home".to_string()) => backup::Definition {
                        repository: repo::Name("local".to_string()),
                        path: backup::Path("/home/user".to_string()),
                        excludes: vec![
                            backup::Exclude("/.local/share/Trash".to_string()),
                            backup::Exclude("/.cache".to_string()),
                        ],
                        exclude_caches: true,
                        exclude_larger_than: Some("1G".to_string()),
                        disable_triggers: false,
                        extra_args: vec!["--one-file-system".to_string()],
                        triggers: vec![
                            trigger::Trigger::Cron(trigger::cron::Cron {
                                cron: "2 * *".to_string(),
                                timezone: trigger::cron::Timezone::Other("Europe/Berlin".to_string())
                            }),
                           trigger::Trigger::Cron(trigger::cron::Cron {
                                cron: "1 * *".to_string(),
                                timezone: trigger::cron::Timezone::Local
                            }),
                        ]
                    },
                    backup::Name("srv".to_string()) => backup::Definition {
                        repository: repo::Name("sftp".to_string()),
                        path: backup::Path("/srv".to_string()),
                        excludes: vec![],
                        exclude_caches: false,
                        exclude_larger_than: None,
                        disable_triggers: true,
                        extra_args: vec![],
                        triggers: vec![]
                    },
                }),
                source: None,
            }
        );
        Ok(())
    }

    #[test]
    fn should_support_underscores_instead_of_dashes_in_settings() -> eyre::Result<()> {
        let input: toml::Value = toml::from_str(
            //language=TOML
            r#"
            [repositories.test]
            url = "/url"
            password = { env_var = "var" }

            [backups.test]
            repository = "test"
            path = "/"
            exclude_caches = true
            exclude_larger_than = "1G"
            extra_args = [""]
            disable_triggers = true
            "#,
        )?;

        let config: Config = input.try_into()?;

        assert_eq!(
            config,
            Config {
                repositories: Repositories(hashmap! {
                    repo::Name("test".to_string()) => repo::Definition {
                        url: repo::Url("/url".to_string()),
                        password: repo::Secret::FromEnvVar { env_var: "var".to_string() },
                        secrets: HashMap::new(),
                    },
                }),
                backups: Backups(hashmap! {
                    backup::Name("test".to_string()) => backup::Definition {
                        repository: repo::Name("test".to_string()),
                        path: backup::Path("/".to_string()),
                        excludes: vec![],
                        exclude_caches: true,
                        exclude_larger_than: Some("1G".to_string()),
                        disable_triggers: true,
                        extra_args: vec!["".to_string()],
                        triggers: vec![]
                    },
                }),
                source: None,
            }
        );
        Ok(())
    }
}
