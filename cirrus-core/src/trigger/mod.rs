use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

#[derive(Debug, PartialEq, Eq, Hash, Clone, Ord, PartialOrd, Copy)]
pub struct NextSchedule(pub PrimitiveDateTime);

#[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
pub struct Trigger(pub schedule_dsl::Schedule);

impl Trigger {
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

#[cfg(test)]
mod tests {
    use super::*;
    use schedule_dsl::Schedule;
    use serde_json::json;

    #[test]
    fn should_deserialize_schedule() {
        let json = json!({
            "at": "12:30"
        });
        let result = serde_json::from_value::<Trigger>(json);

        assert_eq!(
            result.unwrap(),
            Trigger(Schedule::from_time("12:30").unwrap())
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
            Trigger(Schedule::from_time_and_days("6am", "Tuesday").unwrap())
        );
    }
}
