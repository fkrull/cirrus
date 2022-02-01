use super::Schedule;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ScheduleDto {
    at: String,
    every: Option<String>,
}

impl TryFrom<ScheduleDto> for Schedule {
    type Error = crate::parse::ParseError;

    fn try_from(serde_value: ScheduleDto) -> Result<Self, Self::Error> {
        match serde_value.every {
            None => Schedule::from_time(serde_value.at),
            Some(every) => Schedule::from_time_and_days(serde_value.at, every),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod serialize {
        use super::*;
        use serde_json::json;

        #[test]
        fn should_serialize_schedule_with_at() {
            let schedule = Schedule::from_time("10:15\n").unwrap();
            let result = serde_json::to_value(schedule);

            assert_eq!(
                result.unwrap(),
                json!({
                    "at": "10:15\n",
                    "every": null,
                })
            );
        }

        #[test]
        fn should_serialize_schedule_with_at_and_every() {
            let schedule = Schedule::from_time_and_days("6:00", "weekday").unwrap();
            let result = serde_json::to_value(schedule);

            assert_eq!(
                result.unwrap(),
                json!({
                    "at": "6:00",
                    "every": "weekday"
                })
            );
        }
    }

    mod deserialize {
        use super::*;
        use crate::{DayOfWeek, TimeSpec};
        use maplit::btreeset;
        use serde_json::json;

        #[test]
        fn should_deserialize_schedule_with_at() {
            let json = json!({
                "at": "4:33 pm"
            });
            let result = serde_json::from_value::<Schedule>(json);

            assert_eq!(
                result.unwrap(),
                Schedule {
                    days: DayOfWeek::all_days(),
                    times: btreeset![TimeSpec::new(16, 33).unwrap()],
                    every_spec: None,
                    at_spec: "4:33 pm".to_string()
                }
            );
        }

        #[test]
        fn should_deserialize_schedule_with_at_and_every() {
            let json = json!({
                "at": "17:59",
                "every": "weekday except Wednesday"
            });
            let result = serde_json::from_value::<Schedule>(json);

            assert_eq!(
                result.unwrap(),
                Schedule {
                    days: DayOfWeek::weekdays() - DayOfWeek::Wednesday,
                    times: btreeset![TimeSpec::new(17, 59).unwrap()],
                    every_spec: Some("weekday except Wednesday".to_string()),
                    at_spec: "17:59".to_string()
                }
            );
        }

        #[test]
        fn should_not_deserialize_schedule_without_at() {
            let json = json!({
                "every": "day"
            });
            let result = serde_json::from_value::<Schedule>(json);

            assert!(result.is_err());
        }

        #[test]
        fn should_not_deserialize_schedule_with_unknown_field() {
            let json = json!({
                "at": "3:04",
                "unless": "on vacation"
            });
            let result = serde_json::from_value::<Schedule>(json);

            assert!(result.is_err());
        }

        #[test]
        fn should_not_deserialize_schedule_with_invalid_at() {
            let json = json!({
                "at": "50:25",
            });
            let result = serde_json::from_value::<Schedule>(json);

            assert!(result.is_err());
        }

        #[test]
        fn should_not_deserialize_schedule_with_invalid_every() {
            let json = json!({
                "at": "1:45",
                "every": "year"
            });
            let result = serde_json::from_value::<Schedule>(json);

            assert!(result.is_err());
        }
    }
}
