use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct Backups(pub HashMap<backup::Name, backup::Definition>);

impl Backups {
    pub fn get(&self, name: &backup::Name) -> Option<&backup::Definition> {
        self.0.get(name)
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Config {
    pub repositories: Repositories,
    pub backups: Backups,
}

#[derive(Debug, Error)]
#[error("unknown repository '{}'", (self.0).0)]
pub struct UnknownRepository(repo::Name);

#[derive(Debug, Error)]
#[error("unknown backup '{}'", (self.0).0)]
pub struct UnknownBackup(backup::Name);

impl Config {
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

    #[test]
    fn should_parse_complex_config() -> anyhow::Result<()> {
        let input: toml::Value = toml::from_str(
            //language=TOML
            r#"
            [repositories.local]
            url = "/srv/restic-repo"
            password = { env_var = "LOCAL_PASSWORD" }

            [repositories.sftp]
            url = "sftp:user@host:repo/path"
            password = { env_var = "SSH_PASSWORD" }
            
            [repositories.sftp.secrets.UNUSED_SECRET]
            env_var = "SECRET_ENV"

            [backups.home]
            repository = "local"
            path = "/home/user"
            excludes = [
                "/.local/share/Trash",
                "/.cache"
            ]
            exclude_caches = true
            extra_args = ["--one-file-system"]

            # look I don't remember cron syntax
            [[backups.home.triggers]]
            cron = "2 * *"
            timezone = "Europe/Berlin"
            [[backups.home.triggers]]
            cron = "1 * *"

            [backups.srv]
            repository = "sftp"
            path = "/srv"
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
                        extra_args: vec!["--one-file-system".to_string()],
                        triggers: vec![
                            backup::Trigger::Cron {
                                cron: "2 * *".to_string(),
                                timezone: backup::Timezone::Other("Europe/Berlin".to_string())
                            },
                            backup::Trigger::Cron {
                                cron: "1 * *".to_string(),
                                timezone: backup::Timezone::Local
                            },
                        ]
                    },
                    backup::Name("srv".to_string()) => backup::Definition {
                        repository: repo::Name("sftp".to_string()),
                        path: backup::Path("/srv".to_string()),
                        excludes: vec![],
                        extra_args: vec![],
                        triggers: vec![]
                    },
                }),
            }
        );
        Ok(())
    }
}
