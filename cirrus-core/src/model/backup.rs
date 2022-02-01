use crate::{
    model::repo,
    trigger::{NextSchedule, Trigger},
};
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
    pub fn next_schedule(&self, after: OffsetDateTime) -> eyre::Result<Option<NextSchedule>> {
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
        use schedule_dsl::Schedule;
        use time::format_description::well_known::Rfc3339;
        use time::{PrimitiveDateTime, UtcOffset};

        fn local_time(s: &str) -> OffsetDateTime {
            let tz = UtcOffset::current_local_offset().unwrap();
            PrimitiveDateTime::parse(s, &Rfc3339)
                .unwrap()
                .assume_offset(tz)
        }

        #[test]
        fn should_get_next_schedule_from_a_single_trigger() {
            let definition = Definition {
                triggers: vec![Trigger::Schedule(Schedule::from_time("14:00").unwrap())],
                ..Default::default()
            };

            let next = definition
                .next_schedule(local_time("2020-05-17T12:11:16.666Z"))
                .unwrap()
                .unwrap();

            assert_eq!(
                next.0,
                PrimitiveDateTime::parse("2020-05-17T14:00:00Z", &Rfc3339).unwrap()
            );
        }

        #[test]
        fn should_get_first_next_schedule_from_first_trigger() {
            let definition = Definition {
                triggers: vec![
                    Trigger::Schedule(Schedule::from_time("16:00").unwrap()),
                    Trigger::Schedule(Schedule::from_time("17:00").unwrap()),
                ],
                ..Default::default()
            };

            let next = definition
                .next_schedule(local_time("2020-05-17T00:00:00Z"))
                .unwrap()
                .unwrap();

            assert_eq!(
                next.0,
                PrimitiveDateTime::parse("2020-05-17T16:00:00Z", &Rfc3339).unwrap()
            );
        }

        #[test]
        fn should_get_first_next_schedule_from_second_trigger() {
            let definition = Definition {
                triggers: vec![
                    Trigger::Schedule(Schedule::from_time("18:00").unwrap()),
                    Trigger::Schedule(Schedule::from_time("17:00").unwrap()),
                ],
                ..Default::default()
            };

            let next = definition
                .next_schedule(local_time("2020-05-17T00:00:00Z"))
                .unwrap()
                .unwrap();

            assert_eq!(
                next.0,
                PrimitiveDateTime::parse("2020-05-17T17:00:00Z", &Rfc3339).unwrap()
            );
        }

        #[test]
        fn should_get_no_schedule_if_no_triggers() {
            let definition = Definition {
                triggers: vec![],
                ..Default::default()
            };

            let next = definition
                .next_schedule(local_time("2020-05-17T00:00:00Z"))
                .unwrap();

            assert_eq!(next, None);
        }
    }
}
