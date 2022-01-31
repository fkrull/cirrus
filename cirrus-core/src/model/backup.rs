use super::repo;
use crate::trigger::Trigger;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Name(pub String);

#[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct Path(pub String);

#[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Exclude(pub String);

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub struct Definition {
    pub repository: repo::Name,
    pub path: Path,
    #[serde(default)]
    pub excludes: Vec<Exclude>,
    #[serde(default, alias = "exclude_caches")]
    pub exclude_caches: bool,
    #[serde(default, alias = "exclude_larger_than")]
    pub exclude_larger_than: Option<String>,
    #[serde(default, alias = "extra_args")]
    pub extra_args: Vec<String>,
    #[serde(default, alias = "disable_triggers")]
    pub disable_triggers: bool,
    #[serde(default)]
    pub triggers: Vec<Trigger>,
}

impl Definition {
    pub fn next_schedule(&self, after: OffsetDateTime) -> eyre::Result<Option<OffsetDateTime>> {
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

    mod definition {
        use super::*;
        use crate::trigger::cron::{Cron, Timezone};
        use time::format_description::well_known::Rfc3339;

        #[test]
        fn should_get_next_schedule_from_a_single_trigger() {
            let definition = Definition {
                triggers: vec![Trigger::Cron(Cron {
                    cron: "10 * * * *".to_string(),
                    timezone: Timezone::Utc,
                })],
                ..Default::default()
            };

            let next = definition
                .next_schedule(OffsetDateTime::parse("2020-05-17T12:11:16.666Z", &Rfc3339).unwrap())
                .unwrap()
                .unwrap();

            assert_eq!(
                next,
                OffsetDateTime::parse("2020-05-17T13:10:00Z", &Rfc3339).unwrap()
            );
        }

        #[test]
        fn should_get_first_next_schedule_from_first_trigger() {
            let definition = Definition {
                triggers: vec![
                    Trigger::Cron(Cron {
                        cron: "* 16 * * *".to_string(),
                        timezone: Timezone::Utc,
                    }),
                    Trigger::Cron(Cron {
                        cron: "* 17 * * *".to_string(),
                        timezone: Timezone::Utc,
                    }),
                ],
                ..Default::default()
            };

            let next = definition
                .next_schedule(OffsetDateTime::parse("2020-05-17T00:00:00Z", &Rfc3339).unwrap())
                .unwrap()
                .unwrap();

            assert_eq!(
                next,
                OffsetDateTime::parse("2020-05-17T16:00:00Z", &Rfc3339).unwrap()
            );
        }

        #[test]
        fn should_get_first_next_schedule_from_second_trigger() {
            let definition = Definition {
                triggers: vec![
                    Trigger::Cron(Cron {
                        cron: "* 18 * * *".to_string(),
                        timezone: Timezone::Utc,
                    }),
                    Trigger::Cron(Cron {
                        cron: "* 17 * * *".to_string(),
                        timezone: Timezone::Utc,
                    }),
                ],
                ..Default::default()
            };

            let next = definition
                .next_schedule(OffsetDateTime::parse("2020-05-17T00:00:00Z", &Rfc3339).unwrap())
                .unwrap()
                .unwrap();

            assert_eq!(
                next,
                OffsetDateTime::parse("2020-05-17T17:00:00Z", &Rfc3339).unwrap()
            );
        }

        #[test]
        fn should_get_no_schedule_if_no_triggers() {
            let definition = Definition {
                triggers: vec![],
                ..Default::default()
            };

            let next = definition
                .next_schedule(OffsetDateTime::parse("2020-05-17T00:00:00Z", &Rfc3339).unwrap())
                .unwrap();

            assert_eq!(next, None);
        }
    }
}
