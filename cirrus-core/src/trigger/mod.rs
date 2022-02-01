use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

pub mod cron;

#[derive(Debug, PartialEq, Eq, Hash, Clone, Ord, PartialOrd, Copy)]
pub struct NextSchedule(pub PrimitiveDateTime);

#[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Trigger {
    Cron(cron::Cron),
    Schedule(human_schedule::Schedule),
}

impl Trigger {
    pub fn next_schedule(&self, after: time::OffsetDateTime) -> eyre::Result<NextSchedule> {
        match self {
            Trigger::Cron(cron) => {
                let next_schedule = cron.next_schedule(after)?;
                let local_schedule =
                    next_schedule.to_offset(time::UtcOffset::local_offset_at(next_schedule)?);
                Ok(NextSchedule(PrimitiveDateTime::new(
                    local_schedule.date(),
                    local_schedule.time(),
                )))
            }
            Trigger::Schedule(schedule) => {
                let local_offset = time::UtcOffset::local_offset_at(after)?;
                let local_time = after.to_offset(local_offset);
                let wall_time = time::PrimitiveDateTime::new(local_time.date(), local_time.time());
                let next_schedule = schedule
                    .next_schedule(wall_time)
                    .ok_or_else(|| eyre::eyre!("no next schedule"))?;
                Ok(NextSchedule(next_schedule))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn should_deserialize_cron() {
        let json = json!({
            "cron": "2 * *"
        });
        let result = serde_json::from_value::<Trigger>(json);

        assert_eq!(
            result.unwrap(),
            Trigger::Cron(cron::Cron {
                cron: "2 * *".to_owned(),
                timezone: cron::Timezone::default()
            })
        );
    }

    #[test]
    fn should_deserialize_cron_with_timezone() {
        let json = json!({
            "cron": "blub",
            "timezone": "utc"
        });
        let result = serde_json::from_value::<Trigger>(json);

        assert_eq!(
            result.unwrap(),
            Trigger::Cron(cron::Cron {
                cron: "blub".to_owned(),
                timezone: cron::Timezone::Utc
            })
        );
    }

    #[test]
    fn should_deserialize_schedule() {
        let json = json!({
            "at": "12:30"
        });
        let result = serde_json::from_value::<Trigger>(json);

        assert_eq!(
            result.unwrap(),
            Trigger::Schedule(human_schedule::Schedule::from_time("12:30").unwrap())
        );
    }

    #[test]
    fn should_deserialize_schedule_with_days() {
        let json = json!({
            "at": "6am",
            "every": "Tuesday"
        });
        let result = serde_json::from_value::<Trigger>(json);

        assert_eq!(
            result.unwrap(),
            Trigger::Schedule(
                human_schedule::Schedule::from_time_and_days("6am", "Tuesday").unwrap()
            )
        );
    }
}
