use super::repo;
use chrono::DateTime;
use serde::{de, de::Visitor, Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Name(pub String);

#[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize, Default)]
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
    fn serialize<S>(&self, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error>
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

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub struct Definition {
    pub repository: repo::Name,
    pub path: Path,
    #[serde(default)]
    pub excludes: Vec<Exclude>,
    #[serde(default)]
    pub exclude_caches: bool,
    #[serde(default)]
    pub exclude_larger_than: Option<String>,
    #[serde(default)]
    pub extra_args: Vec<String>,
    #[serde(default)]
    pub disable_triggers: bool,
    #[serde(default)]
    pub triggers: Vec<Trigger>,
}

impl Trigger {
    pub fn next_schedule(
        &self,
        after: DateTime<chrono::Utc>,
    ) -> eyre::Result<DateTime<chrono::Utc>> {
        match self {
            Trigger::Cron {
                cron,
                timezone: Timezone::Utc,
            } => Ok(cron_parser::parse(cron, &after)?),
            Trigger::Cron {
                cron,
                timezone: Timezone::Local,
            } => Ok(
                cron_parser::parse(cron, &after.with_timezone(&chrono::Local))?
                    .with_timezone(&chrono::Utc),
            ),
            Trigger::Cron {
                timezone: Timezone::Other(_),
                ..
            } => {
                // TODO: arbitrary timezones?
                Err(eyre::eyre!("arbitrary timezones aren't supported (yet?)"))
            }
        }
    }
}

impl Definition {
    pub fn next_schedule(
        &self,
        after: DateTime<chrono::Utc>,
    ) -> eyre::Result<Option<DateTime<chrono::Utc>>> {
        use std::cmp::min;

        self.triggers
            .iter()
            .map(|trigger| trigger.next_schedule(after))
            .try_fold(None, |acc, next| {
                let next = next?;
                Ok(acc
                    .map(|schedule| min(schedule, next))
                    .or_else(|| Some(next)))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod timezone {
        use super::*;

        #[test]
        fn should_deserialize_utc_timezone() {
            let tz: Timezone = serde_json::from_str(r#""utc""#).unwrap();
            assert_eq!(tz, Timezone::Utc);
        }

        #[test]
        fn should_deserialize_local_timezone() {
            let tz: Timezone = serde_json::from_str(r#""local""#).unwrap();
            assert_eq!(tz, Timezone::Local);
        }

        #[test]
        fn should_deserialize_other_timezone() {
            let tz: Timezone = serde_json::from_str(r#""Antarctica/Troll""#).unwrap();
            assert_eq!(tz, Timezone::Other("Antarctica/Troll".to_string()));
        }

        #[test]
        fn should_serialize_utc_timezone() {
            let s = serde_json::to_string(&Timezone::Utc).unwrap();
            assert_eq!(&s, r#""utc""#);
        }

        #[test]
        fn should_serialize_local_timezone() {
            let s = serde_json::to_string(&Timezone::Local).unwrap();
            assert_eq!(&s, r#""local""#);
        }

        #[test]
        fn should_serialize_other_timezone() {
            let s =
                serde_json::to_string(&Timezone::Other("Africa/Casablanca".to_string())).unwrap();
            assert_eq!(&s, r#""Africa/Casablanca""#);
        }
    }

    mod trigger {
        use super::*;
        use chrono::{NaiveDateTime, TimeZone};
        use std::str::FromStr;

        #[test]
        fn should_get_next_schedule_for_cron_expression() {
            let trigger = Trigger::Cron {
                cron: "30 10 * * *".to_string(),
                timezone: Timezone::Utc,
            };
            let next = trigger
                .next_schedule(DateTime::from_str("2020-05-14T9:56:13.123Z").unwrap())
                .unwrap();
            assert_eq!(
                next,
                DateTime::from_str("2020-05-14T10:30:00Z").unwrap() as DateTime<chrono::Utc>
            );
        }

        #[test]
        fn should_get_next_schedule_for_another_cron_expression() {
            let trigger = Trigger::Cron {
                cron: "0 */6 * * *".to_string(),
                timezone: Timezone::Utc,
            };
            let next = trigger
                .next_schedule(DateTime::from_str("2020-05-15T00:04:52.123Z").unwrap())
                .unwrap();
            assert_eq!(
                next,
                DateTime::from_str("2020-05-15T06:00:00Z").unwrap() as DateTime<chrono::Utc>
            );
        }

        #[test]
        fn should_get_next_schedule_for_a_cron_expression_using_local_time() {
            let trigger = Trigger::Cron {
                cron: "34 13 15 5 *".to_string(),
                timezone: Timezone::Local,
            };
            let local = chrono::Local
                .from_local_datetime(&NaiveDateTime::from_str("2020-04-16T07:13:31.666").unwrap())
                .unwrap();
            let expected_local = chrono::Local
                .from_local_datetime(&NaiveDateTime::from_str("2020-05-15T13:34:00").unwrap())
                .unwrap();

            let next = trigger
                .next_schedule(local.with_timezone(&chrono::Utc))
                .unwrap();

            assert_eq!(next, expected_local.with_timezone(&chrono::Utc));
        }
    }

    mod definition {
        use super::*;
        use std::str::FromStr;

        #[test]
        fn should_get_next_schedule_from_a_single_trigger() {
            let definition = Definition {
                triggers: vec![Trigger::Cron {
                    cron: "10 * * * *".to_string(),
                    timezone: Timezone::Utc,
                }],
                ..Default::default()
            };

            let next = definition
                .next_schedule(DateTime::from_str("2020-05-17T12:11:16.666Z").unwrap())
                .unwrap()
                .unwrap();

            assert_eq!(
                next,
                DateTime::from_str("2020-05-17T13:10:00Z").unwrap() as DateTime<chrono::Utc>
            );
        }

        #[test]
        fn should_get_first_next_schedule_from_first_trigger() {
            let definition = Definition {
                triggers: vec![
                    Trigger::Cron {
                        cron: "* 16 * * *".to_string(),
                        timezone: Timezone::Utc,
                    },
                    Trigger::Cron {
                        cron: "* 17 * * *".to_string(),
                        timezone: Timezone::Utc,
                    },
                ],
                ..Default::default()
            };

            let next = definition
                .next_schedule(DateTime::from_str("2020-05-17T00:00:00Z").unwrap())
                .unwrap()
                .unwrap();

            assert_eq!(
                next,
                DateTime::from_str("2020-05-17T16:00:00Z").unwrap() as DateTime<chrono::Utc>
            );
        }

        #[test]
        fn should_get_first_next_schedule_from_second_trigger() {
            let definition = Definition {
                triggers: vec![
                    Trigger::Cron {
                        cron: "* 18 * * *".to_string(),
                        timezone: Timezone::Utc,
                    },
                    Trigger::Cron {
                        cron: "* 17 * * *".to_string(),
                        timezone: Timezone::Utc,
                    },
                ],
                ..Default::default()
            };

            let next = definition
                .next_schedule(DateTime::from_str("2020-05-17T00:00:00Z").unwrap())
                .unwrap()
                .unwrap();

            assert_eq!(
                next,
                DateTime::from_str("2020-05-17T17:00:00Z").unwrap() as DateTime<chrono::Utc>
            );
        }

        #[test]
        fn should_get_no_schedule_if_no_triggers() {
            let definition = Definition {
                triggers: vec![],
                ..Default::default()
            };

            let next = definition
                .next_schedule(DateTime::from_str("2020-05-17T00:00:00Z").unwrap())
                .unwrap();

            assert_eq!(next, None);
        }
    }
}
