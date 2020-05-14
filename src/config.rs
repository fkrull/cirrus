use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod repo {
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;

    #[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
    #[serde(untagged)]
    pub enum Secret {
        FromEnvVar { env_var: String },
    }

    #[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
    #[serde(transparent)]
    pub struct Name(pub String);

    #[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
    #[serde(transparent)]
    pub struct Url(pub String);

    #[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
    #[serde(transparent)]
    pub struct SecretName(pub String);

    #[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
    pub struct Definition {
        pub url: Url,
        pub password: Secret,
        #[serde(default)]
        pub secrets: HashMap<SecretName, Secret>,
    }
}

pub mod backup {
    use super::repo;
    use anyhow::anyhow;
    use chrono::{DateTime, Local, Utc};
    use serde::{de, de::Visitor, Deserialize, Deserializer, Serialize, Serializer};

    #[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
    #[serde(transparent)]
    pub struct Name(pub String);

    #[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
    #[serde(transparent)]
    pub struct Path(pub String);

    #[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
    #[serde(transparent)]
    pub struct Exclude(pub String);

    #[derive(Debug, PartialEq, Eq, Hash, Clone)]
    pub enum Timezone {
        Utc,
        Local,
        Other(String),
    }

    impl Serialize for Timezone {
        fn serialize<S>(
            &self,
            serializer: S,
        ) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error>
        where
            S: Serializer,
        {
            let ser = match self {
                Timezone::Utc => "utc",
                Timezone::Local => "local",
                Timezone::Other(s) => s,
            };
            serializer.serialize_str(ser)
        }
    }

    impl Timezone {
        fn match_tz(s: &str) -> Option<Timezone> {
            match s {
                "utc" => Some(Timezone::Utc),
                "local" => Some(Timezone::Local),
                _ => None,
            }
        }
    }

    struct TimezoneVisitor;

    impl<'de> Visitor<'de> for TimezoneVisitor {
        type Value = Timezone;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(formatter, r#""utc", "local", or the name of a time zone"#)
        }

        fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            let tz = Timezone::match_tz(s).unwrap_or_else(|| Timezone::Other(s.to_string()));
            Ok(tz)
        }

        fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            let tz = Timezone::match_tz(&v).unwrap_or_else(|| Timezone::Other(v));
            Ok(tz)
        }
    }

    impl<'de> Deserialize<'de> for Timezone {
        fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
        where
            D: Deserializer<'de>,
        {
            deserializer.deserialize_string(TimezoneVisitor)
        }
    }

    impl Default for Timezone {
        fn default() -> Self {
            Timezone::Local
        }
    }

    #[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
    #[serde(untagged)]
    pub enum Trigger {
        Cron {
            cron: String,
            #[serde(default)]
            timezone: Timezone,
        },
    }

    #[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
    pub struct Definition {
        pub repository: repo::Name,
        pub path: Path,
        #[serde(default)]
        pub excludes: Vec<Exclude>,
        #[serde(default)]
        pub extra_args: Vec<String>,
        pub triggers: Vec<Trigger>,
    }

    impl Trigger {
        pub fn next_schedule(&self, after: DateTime<Utc>) -> anyhow::Result<DateTime<Utc>> {
            match self {
                Trigger::Cron {
                    cron,
                    timezone: Timezone::Utc,
                } => Ok(cron_parser::parse(cron, &after)?),
                Trigger::Cron {
                    cron,
                    timezone: Timezone::Local,
                } => {
                    let x = cron_parser::parse(cron, &after.with_timezone(&Local))?;
                    Ok(x.with_timezone(&Utc))
                }
                Trigger::Cron {
                    timezone: Timezone::Other(_),
                    ..
                } => {
                    // TODO: arbitrary timezones?
                    Err(anyhow!("arbitrary timezones aren't supported (yet?)"))
                }
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct Repositories(HashMap<repo::Name, repo::Definition>);

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct Backups(HashMap<backup::Name, backup::Definition>);

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Config {
    pub repositories: Repositories,
    pub backups: Backups,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, Utc};
    use maplit::hashmap;
    use std::str::FromStr;

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
            
            [repositories.sftp.secrets.UNUSED_SECRET]
            env_var = "SECRET_ENV"

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
            timezone = "Europe/Berlin"
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
        )
    }

    mod timezone {
        use super::*;

        #[test]
        fn should_deserialize_utc_timezone() {
            let tz: backup::Timezone = serde_json::from_str(r#""utc""#).unwrap();
            assert_eq!(tz, backup::Timezone::Utc);
        }

        #[test]
        fn should_deserialize_local_timezone() {
            let tz: backup::Timezone = serde_json::from_str(r#""local""#).unwrap();
            assert_eq!(tz, backup::Timezone::Local);
        }

        #[test]
        fn should_deserialize_other_timezone() {
            let tz: backup::Timezone = serde_json::from_str(r#""Antarctica/Troll""#).unwrap();
            assert_eq!(tz, backup::Timezone::Other("Antarctica/Troll".to_string()));
        }

        #[test]
        fn should_serialize_utc_timezone() {
            let s = serde_json::to_string(&backup::Timezone::Utc).unwrap();
            assert_eq!(&s, r#""utc""#);
        }

        #[test]
        fn should_serialize_local_timezone() {
            let s = serde_json::to_string(&backup::Timezone::Local).unwrap();
            assert_eq!(&s, r#""local""#);
        }

        #[test]
        fn should_serialize_other_timezone() {
            let s =
                serde_json::to_string(&backup::Timezone::Other("Africa/Casablanca".to_string()))
                    .unwrap();
            assert_eq!(&s, r#""Africa/Casablanca""#);
        }
    }

    #[test]
    fn should_get_next_schedule_for_cron_expression() {
        let trigger = backup::Trigger::Cron {
            cron: "30 10 * * *".to_string(),
            timezone: backup::Timezone::Utc,
        };
        let next = trigger
            .next_schedule(DateTime::from_str("2020-05-14T9:56:13.123Z").unwrap())
            .unwrap();
        assert_eq!(
            next,
            DateTime::from_str("2020-05-14T10:30:00Z").unwrap() as DateTime<Utc>
        );
    }

    #[test]
    fn should_get_next_schedule_for_another_cron_expression() {
        let trigger = backup::Trigger::Cron {
            cron: "0 */6 * * *".to_string(),
            timezone: backup::Timezone::Utc,
        };
        let next = trigger
            .next_schedule(DateTime::from_str("2020-05-15T00:04:52.123Z").unwrap())
            .unwrap();
        assert_eq!(
            next,
            DateTime::from_str("2020-05-15T06:00:00Z").unwrap() as DateTime<Utc>
        );
    }
}
