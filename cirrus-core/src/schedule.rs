use crate::config::backup;
use std::cmp::min;
use time::{OffsetDateTime, PrimitiveDateTime};

#[derive(Debug, PartialEq, Eq, Hash, Clone, Ord, PartialOrd, Copy)]
pub struct NextSchedule(pub PrimitiveDateTime);

impl backup::Trigger {
    pub fn next_schedule(&self, after: time::OffsetDateTime) -> eyre::Result<NextSchedule> {
        let local_offset = time::UtcOffset::local_offset_at(after)?;
        let local_time = after.to_offset(local_offset);
        let wall_time = time::PrimitiveDateTime::new(local_time.date(), local_time.time());
        let next_schedule = self
            .0
            .next_schedule(wall_time)
            .ok_or_else(|| eyre::eyre!("no next schedule"))?;
        Ok(NextSchedule(next_schedule))
    }
}

impl backup::Definition {
    pub fn next_schedule(&self, after: OffsetDateTime) -> eyre::Result<Option<NextSchedule>> {
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
        use time::{PrimitiveDateTime, UtcOffset};

        fn local_offset_time(s: &str) -> OffsetDateTime {
            let tz = UtcOffset::current_local_offset().unwrap();
            local_time(s).assume_offset(tz)
        }

        fn local_time(s: &str) -> PrimitiveDateTime {
            PrimitiveDateTime::parse(s, &time::format_description::well_known::Rfc3339).unwrap()
        }

        #[test]
        fn should_get_next_schedule_from_a_single_trigger() {
            let definition = backup::Definition {
                triggers: vec![backup::Trigger(Schedule::from_time("14:00").unwrap())],
                ..Default::default()
            };

            let next = definition
                .next_schedule(local_offset_time("2020-05-17T12:11:16.666Z"))
                .unwrap()
                .unwrap();

            assert_eq!(next, NextSchedule(local_time("2020-05-17T14:00:00Z")));
        }

        #[test]
        fn should_get_first_next_schedule_from_first_trigger() {
            let definition = backup::Definition {
                triggers: vec![
                    backup::Trigger(Schedule::from_time("16:00").unwrap()),
                    backup::Trigger(Schedule::from_time("17:00").unwrap()),
                ],
                ..Default::default()
            };

            let next = definition
                .next_schedule(local_offset_time("2020-05-17T00:00:00Z"))
                .unwrap()
                .unwrap();

            assert_eq!(next, NextSchedule(local_time("2020-05-17T16:00:00Z")));
        }

        #[test]
        fn should_get_first_next_schedule_from_second_trigger() {
            let definition = backup::Definition {
                triggers: vec![
                    backup::Trigger(Schedule::from_time("18:00").unwrap()),
                    backup::Trigger(Schedule::from_time("17:00").unwrap()),
                ],
                ..Default::default()
            };

            let next = definition
                .next_schedule(local_offset_time("2020-05-17T00:00:00Z"))
                .unwrap()
                .unwrap();

            assert_eq!(next, NextSchedule(local_time("2020-05-17T17:00:00Z")));
        }

        #[test]
        fn should_get_no_schedule_if_no_triggers() {
            let definition = backup::Definition {
                triggers: vec![],
                ..Default::default()
            };

            let next = definition
                .next_schedule(local_offset_time("2020-05-17T00:00:00Z"))
                .unwrap();

            assert_eq!(next, None);
        }
    }
}
