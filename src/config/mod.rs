use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod backup;
pub mod repo;

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Config {
    pub repositories: HashMap<repo::Name, repo::Definition>,
    pub backups: HashMap<backup::Name, backup::Definition>,
}

#[cfg(test)]
mod tests {
    use maplit::hashmap;

    use super::*;

    #[test]
    fn should_parse_complex_config() {
        let input: toml::Value = toml::from_str(
            //language=TOML
            r#"
            [repositories.local]
            url = "/srv/restic-repo"
            password = { env_var = "LOCAL_PASSWORD" }

            [repositories.sftp]
            url = "sftp:user@host:repo/path"
            password = { env_var = "SSH_PASSWORD" }

            [backups.home]
            repository = "local"
            path = "/home/user"
            excludes = [
                "/.local/share/Trash",
                "/.cache"
            ]
            extra_args = ["--one-file-system"]

            # look I don't remember cron syntax
            [[backups.home.triggers]]
            cron = "2 * *"
            [[backups.home.triggers]]
            cron = "1 * *"

            [backups.srv]
            repository = "sftp"
            path = "/srv"
            triggers = []
            "#,
        )
        .unwrap();

        let config: Config = input.try_into().unwrap();

        assert_eq!(
            config,
            Config {
                repositories: hashmap! {
                    repo::Name("local".to_string()) => repo::Definition {
                        url: repo::Url("/srv/restic-repo".to_string()),
                        password: repo::Password::FromEnvVar { env_var: "LOCAL_PASSWORD".to_string() },
                    },
                    repo::Name("sftp".to_string()) => repo::Definition {
                        url: repo::Url("sftp:user@host:repo/path".to_string()),
                        password: repo::Password::FromEnvVar { env_var: "SSH_PASSWORD".to_string() },
                    },
                },
                backups: hashmap! {
                    backup::Name("home".to_string()) => backup::Definition {
                        repository: repo::Name("local".to_string()),
                        path: backup::Path("/home/user".to_string()),
                        excludes: vec![
                            backup::Exclude("/.local/share/Trash".to_string()),
                            backup::Exclude("/.cache".to_string()),
                        ],
                        extra_args: vec!["--one-file-system".to_string()],
                        triggers: vec![
                            backup::Trigger::Cron { cron: "2 * *".to_string() },
                            backup::Trigger::Cron { cron: "1 * *".to_string() },
                        ]
                    },
                    backup::Name("srv".to_string()) => backup::Definition {
                        repository: repo::Name("sftp".to_string()),
                        path: backup::Path("/srv".to_string()),
                        excludes: vec![],
                        extra_args: vec![],
                        triggers: vec![]
                    },
                },
            }
        )
    }
}
