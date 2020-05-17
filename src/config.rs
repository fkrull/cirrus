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

    #[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Serialize, Deserialize)]
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
    use anyhow::Context;
    use chrono::{DateTime, Local, Utc};
    use serde::{de, de::Visitor, Deserialize, Deserializer, Serialize, Serializer};
    use std::cmp::min;

    #[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Serialize, Deserialize)]
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
                    let x = cron_parser::parse(cron, &after.with_timezone(&Local))
                        .context(format!("invalid cron expression '{}'", cron))?;
                    Ok(x.with_timezone(&Utc))
                }
                Trigger::Cron {
                    timezone: Timezone::Other(_),
                    ..
                } => {
                    // TODO: arbitrary timezones?
                    Err(anyhow::anyhow!(
                        "arbitrary timezones aren't supported (yet?)"
                    ))
                }
            }
        }
    }

    impl Default for Definition {
        fn default() -> Self {
            Definition {
                repository: repo::Name(String::new()),
                path: Path(String::new()),
                excludes: vec![],
                extra_args: vec![],
                triggers: vec![],
            }
        }
    }

    impl Definition {
        pub fn next_schedule(&self, after: DateTime<Utc>) -> anyhow::Result<Option<DateTime<Utc>>> {
            self.triggers
                .iter()
                .map(|trigger| trigger.next_schedule(after))
                .try_fold(None, |acc, next| {
                    let next = next?;
                    Ok(Some(match acc {
                        Some(schedule) => min(schedule, next),
                        None => next,
                    }))
                })
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct Repositories(pub HashMap<repo::Name, repo::Definition>);

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct Backups(pub HashMap<backup::Name, backup::Definition>);

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Config {
    pub repositories: Repositories,
    pub backups: Backups,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, TimeZone, Utc};
    use maplit::hashmap;
    use std::str::FromStr;

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

    mod timezone {
        use super::*;

        #[test]
        fn should_deserialize_utc_timezone() -> anyhow::Result<()> {
            let tz: backup::Timezone = serde_json::from_str(r#""utc""#)?;
            assert_eq!(tz, backup::Timezone::Utc);
            Ok(())
        }

        #[test]
        fn should_deserialize_local_timezone() -> anyhow::Result<()> {
            let tz: backup::Timezone = serde_json::from_str(r#""local""#)?;
            assert_eq!(tz, backup::Timezone::Local);
            Ok(())
        }

        #[test]
        fn should_deserialize_other_timezone() -> anyhow::Result<()> {
            let tz: backup::Timezone = serde_json::from_str(r#""Antarctica/Troll""#)?;
            assert_eq!(tz, backup::Timezone::Other("Antarctica/Troll".to_string()));
            Ok(())
        }

        #[test]
        fn should_serialize_utc_timezone() -> anyhow::Result<()> {
            let s = serde_json::to_string(&backup::Timezone::Utc)?;
            assert_eq!(&s, r#""utc""#);
            Ok(())
        }

        #[test]
        fn should_serialize_local_timezone() -> anyhow::Result<()> {
            let s = serde_json::to_string(&backup::Timezone::Local)?;
            assert_eq!(&s, r#""local""#);
            Ok(())
        }

        #[test]
        fn should_serialize_other_timezone() -> anyhow::Result<()> {
            let s =
                serde_json::to_string(&backup::Timezone::Other("Africa/Casablanca".to_string()))?;
            assert_eq!(&s, r#""Africa/Casablanca""#);
            Ok(())
        }
    }

    mod trigger {
        use super::*;
        use chrono::{Local, NaiveDateTime};

        #[test]
        fn should_get_next_schedule_for_cron_expression() -> anyhow::Result<()> {
            let trigger = backup::Trigger::Cron {
                cron: "30 10 * * *".to_string(),
                timezone: backup::Timezone::Utc,
            };
            let next = trigger.next_schedule(DateTime::from_str("2020-05-14T9:56:13.123Z")?)?;
            assert_eq!(
                next,
                DateTime::from_str("2020-05-14T10:30:00Z")? as DateTime<Utc>
            );
            Ok(())
        }

        #[test]
        fn should_get_next_schedule_for_another_cron_expression() -> anyhow::Result<()> {
            let trigger = backup::Trigger::Cron {
                cron: "0 */6 * * *".to_string(),
                timezone: backup::Timezone::Utc,
            };
            let next = trigger.next_schedule(DateTime::from_str("2020-05-15T00:04:52.123Z")?)?;
            assert_eq!(
                next,
                DateTime::from_str("2020-05-15T06:00:00Z")? as DateTime<Utc>
            );
            Ok(())
        }

        #[test]
        fn should_get_next_schedule_for_a_cron_expression_using_local_time() -> anyhow::Result<()> {
            let trigger = backup::Trigger::Cron {
                cron: "34 13 15 5 *".to_string(),
                timezone: backup::Timezone::Local,
            };
            let local = Local
                .from_local_datetime(&NaiveDateTime::from_str("2020-04-16T07:13:31.666")?)
                .unwrap();
            let expected_local = Local
                .from_local_datetime(&NaiveDateTime::from_str("2020-05-15T13:34:00")?)
                .unwrap();

            let next = trigger.next_schedule(local.with_timezone(&Utc))?;

            assert_eq!(next, expected_local.with_timezone(&Utc));
            Ok(())
        }
    }

    mod definition {
        use super::*;

        #[test]
        fn should_get_next_schedule_from_a_single_trigger() -> anyhow::Result<()> {
            let definition = backup::Definition {
                triggers: vec![backup::Trigger::Cron {
                    cron: "10 * * * *".to_string(),
                    timezone: backup::Timezone::Utc,
                }],
                ..Default::default()
            };

            let next = definition
                .next_schedule(DateTime::from_str("2020-05-17T12:11:16.666Z")?)?
                .unwrap();

            assert_eq!(
                next,
                DateTime::from_str("2020-05-17T13:10:00Z")? as DateTime<Utc>
            );
            Ok(())
        }

        #[test]
        fn should_get_first_next_schedule_from_first_trigger() -> anyhow::Result<()> {
            let definition = backup::Definition {
                triggers: vec![
                    backup::Trigger::Cron {
                        cron: "* 16 * * *".to_string(),
                        timezone: backup::Timezone::Utc,
                    },
                    backup::Trigger::Cron {
                        cron: "* 17 * * *".to_string(),
                        timezone: backup::Timezone::Utc,
                    },
                ],
                ..Default::default()
            };

            let next = definition
                .next_schedule(DateTime::from_str("2020-05-17T00:00:00Z")?)?
                .unwrap();

            assert_eq!(
                next,
                DateTime::from_str("2020-05-17T16:00:00Z")? as DateTime<Utc>
            );
            Ok(())
        }

        #[test]
        fn should_get_first_next_schedule_from_second_trigger() -> anyhow::Result<()> {
            let definition = backup::Definition {
                triggers: vec![
                    backup::Trigger::Cron {
                        cron: "* 18 * * *".to_string(),
                        timezone: backup::Timezone::Utc,
                    },
                    backup::Trigger::Cron {
                        cron: "* 17 * * *".to_string(),
                        timezone: backup::Timezone::Utc,
                    },
                ],
                ..Default::default()
            };

            let next = definition
                .next_schedule(DateTime::from_str("2020-05-17T00:00:00Z")?)?
                .unwrap();

            assert_eq!(
                next,
                DateTime::from_str("2020-05-17T17:00:00Z")? as DateTime<Utc>
            );
            Ok(())
        }

        #[test]
        fn should_get_no_schedule_if_no_triggers() -> anyhow::Result<()> {
            let definition = backup::Definition {
                triggers: vec![],
                ..Default::default()
            };

            let next = definition.next_schedule(DateTime::from_str("2020-05-17T00:00:00Z")?)?;

            assert_eq!(next, None);
            Ok(())
        }
    }
}
