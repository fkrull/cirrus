use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

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

#[derive(Debug, thiserror::Error)]
pub enum ConfigLoadError {
    #[error("invalid configuration string")]
    InvalidConfigString(String, #[source] eyre::Report),
    #[error("invalid configuration file {}", .0.display())]
    InvalidConfigFile(PathBuf, #[source] eyre::Report),
    #[error("i/o error reading configuration file {}", .0.display())]
    IoError(PathBuf, std::io::Error),
}

#[derive(Debug, thiserror::Error)]
#[error("unknown repository '{}'", (self.0).0)]
pub struct UnknownRepository(repo::Name);

#[derive(Debug, thiserror::Error)]
#[error("unknown backup '{}'", (self.0).0)]
pub struct UnknownBackup(backup::Name);

impl Config {
    pub fn parse(s: &str) -> Result<Config, ConfigLoadError> {
        toml::from_str(s).map_err(|e| ConfigLoadError::InvalidConfigString(s.to_owned(), e.into()))
    }

    pub async fn parse_file(p: &Path) -> Result<Config, ConfigLoadError> {
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
    use maplit::hashmap;
    use std::time::Duration;

    #[test]
    fn should_parse_complex_config() {
        let input: toml::Value = toml::from_str(
            //language=TOML
            r#"
            [repositories.local]
            url = "/srv/restic-repo"
            password = { env-var = "LOCAL_PASSWORD" }

            [repositories.sftp]
            url = "sftp:user@host:repo/path"
            parallel-jobs = 6
            build-index = "6 months"
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
            ignore-unreadable-source-files = true
            extra-args = ["--one-file-system"]

            [[backups.home.triggers]]
            at = "16:00"
            every = "weekday"
            [[backups.home.triggers]]
            at = "4am"

            [backups.srv]
            repository = "sftp"
            path = "/srv"
            disable-triggers = true
            triggers = []
            "#,
        )
        .unwrap();

        let config: Config = input.try_into().unwrap();

        assert_eq!(
            config,
            Config {
                repositories: Repositories(hashmap! {
                    repo::Name("local".to_string()) => repo::Definition {
                        url: repo::Url("/srv/restic-repo".to_string()),
                        parallel_jobs: None,
                        build_index: None,
                        password: repo::Secret::FromEnvVar { env_var: "LOCAL_PASSWORD".to_string() },
                        secrets: HashMap::new(),
                    },
                    repo::Name("sftp".to_string()) => repo::Definition {
                        url: repo::Url("sftp:user@host:repo/path".to_string()),
                        parallel_jobs: Some(6),
                        build_index: Some(humantime_serde::re::humantime::parse_duration("6 months").unwrap()),
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
                        ignore_unreadable_source_files: true,
                        disable_triggers: false,
                        extra_args: vec!["--one-file-system".to_string()],
                        triggers: vec![
                            backup::Trigger(
                                schedule_dsl::Schedule::from_time_and_days("16:00", "weekday").unwrap()
                            ),
                            backup::Trigger(schedule_dsl::Schedule::from_time("4am").unwrap()),
                        ]
                    },
                    backup::Name("srv".to_string()) => backup::Definition {
                        repository: repo::Name("sftp".to_string()),
                        path: backup::Path("/srv".to_string()),
                        excludes: vec![],
                        exclude_caches: false,
                        exclude_larger_than: None,
                        ignore_unreadable_source_files: false,
                        disable_triggers: true,
                        extra_args: vec![],
                        triggers: vec![]
                    },
                }),
                source: None,
            }
        );
    }

    #[test]
    fn should_support_underscores_instead_of_dashes_in_settings() {
        let input: toml::Value = toml::from_str(
            //language=TOML
            r#"
            [repositories.test]
            url = "/url"
            parallel_jobs = 8
            build_index = "1s"
            password = { env_var = "var" }

            [backups.test]
            repository = "test"
            path = "/"
            exclude_caches = true
            exclude_larger_than = "1G"
            ignore_unreadable_source_files = true
            extra_args = [""]
            disable_triggers = true
            "#,
        )
        .unwrap();

        let config: Config = input.try_into().unwrap();

        assert_eq!(
            config,
            Config {
                repositories: Repositories(hashmap! {
                    repo::Name("test".to_string()) => repo::Definition {
                        url: repo::Url("/url".to_string()),
                        parallel_jobs: Some(8),
                        build_index: Some(Duration::from_secs(1)),
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
                        ignore_unreadable_source_files: true,
                        disable_triggers: true,
                        extra_args: vec!["".to_string()],
                        triggers: vec![]
                    },
                }),
                source: None,
            }
        );
    }
}
