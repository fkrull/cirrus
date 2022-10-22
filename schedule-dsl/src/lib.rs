use enumset::EnumSet;
use std::collections::BTreeSet;

pub mod parse;
#[cfg(feature = "time")]
mod schedule;
#[cfg(feature = "serde")]
mod serde;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, enumset::EnumSetType)]
#[enumset(no_super_impls)]
pub enum DayOfWeek {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}

impl DayOfWeek {
    pub fn weekdays() -> EnumSet<DayOfWeek> {
        use DayOfWeek::*;
        Monday | Tuesday | Wednesday | Thursday | Friday
    }

    pub fn weekend() -> EnumSet<DayOfWeek> {
        use DayOfWeek::*;
        Saturday | Sunday
    }

    pub fn all_days() -> EnumSet<DayOfWeek> {
        DayOfWeek::weekdays() | DayOfWeek::weekend()
    }
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum TimeSpecOutOfRange {
    #[error("hour value {0} out of range [0,23]")]
    HourOutOfRange(u32),
    #[error("minute value {0} out of range [0,59]")]
    MinuteOutOfRange(u32),
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct TimeSpec {
    hour: u32,
    minute: u32,
}

impl TimeSpec {
    pub fn new(hour: u32, minute: u32) -> Result<TimeSpec, TimeSpecOutOfRange> {
        if hour > 23 {
            Err(TimeSpecOutOfRange::HourOutOfRange(hour))
        } else if minute > 59 {
            Err(TimeSpecOutOfRange::MinuteOutOfRange(minute))
        } else {
            Ok(TimeSpec { hour, minute })
        }
    }

    pub fn hour(&self) -> u32 {
        self.hour
    }

    pub fn minute(&self) -> u32 {
        self.minute
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
#[cfg_attr(feature = "serde", derive(::serde::Deserialize, ::serde::Serialize))]
#[cfg_attr(feature = "serde", serde(try_from = "serde::ScheduleDto"))]
pub struct Schedule {
    #[cfg_attr(feature = "serde", serde(skip_serializing))]
    days: EnumSet<DayOfWeek>,
    #[cfg_attr(feature = "serde", serde(skip_serializing))]
    times: BTreeSet<TimeSpec>,
    #[cfg_attr(feature = "serde", serde(rename(serialize = "every")))]
    every_spec: Option<String>,
    #[cfg_attr(feature = "serde", serde(rename(serialize = "at")))]
    at_spec: String,
}

impl Schedule {
    pub fn from_time(at_spec: impl Into<String>) -> Result<Schedule, parse::ParseError> {
        Schedule::_from_time(at_spec.into())
    }

    fn _from_time(at_spec: String) -> Result<Schedule, parse::ParseError> {
        let times = parse::parse_at_spec(&at_spec)?;
        assert!(!times.is_empty());
        Ok(Schedule {
            days: DayOfWeek::all_days(),
            times,
            every_spec: None,
            at_spec,
        })
    }

    pub fn from_time_and_days(
        at_spec: impl Into<String>,
        every_spec: impl Into<String>,
    ) -> Result<Schedule, parse::ParseError> {
        Schedule::_from_time_and_days(at_spec.into(), every_spec.into())
    }

    fn _from_time_and_days(
        at_spec: String,
        every_spec: String,
    ) -> Result<Schedule, parse::ParseError> {
        let times = parse::parse_at_spec(&at_spec)?;
        assert!(!times.is_empty());
        let days = parse::parse_every_spec(&every_spec)?;
        assert!(!days.is_empty());
        Ok(Schedule {
            days,
            times,
            every_spec: Some(every_spec),
            at_spec,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod time_spec {
        use super::*;

        #[test]
        fn should_create_zero_time_spec() {
            let result = TimeSpec::new(0, 0);

            assert_eq!(result.unwrap(), TimeSpec { hour: 0, minute: 0 });
        }

        #[test]
        fn should_create_max_time_spec() {
            let result = TimeSpec::new(23, 59);

            assert_eq!(
                result.unwrap(),
                TimeSpec {
                    hour: 23,
                    minute: 59
                }
            );
        }

        #[test]
        fn should_check_hour_range() {
            let result = TimeSpec::new(24, 30);

            assert!(matches!(
                result,
                Err(TimeSpecOutOfRange::HourOutOfRange(24))
            ));
        }

        #[test]
        fn should_check_minute_range() {
            let result = TimeSpec::new(12, 60);

            assert!(matches!(
                result,
                Err(TimeSpecOutOfRange::MinuteOutOfRange(60))
            ));
        }
    }

    mod schedule {
        use super::*;
        use maplit::btreeset;

        #[test]
        fn should_parse_time() {
            let result = Schedule::from_time("14:30 and 5 am\n");

            assert_eq!(
                result.unwrap(),
                Schedule {
                    days: DayOfWeek::all_days(),
                    times: btreeset![TimeSpec::new(14, 30).unwrap(), TimeSpec::new(5, 0).unwrap()],
                    every_spec: None,
                    at_spec: "14:30 and 5 am\n".to_string()
                }
            );
        }

        #[test]
        fn should_parse_time_and_days() {
            let result = Schedule::from_time_and_days("6pm", "monday, and thursday");

            assert_eq!(
                result.unwrap(),
                Schedule {
                    days: DayOfWeek::Monday | DayOfWeek::Thursday,
                    times: btreeset![TimeSpec::new(18, 0).unwrap()],
                    every_spec: Some("monday, and thursday".to_string()),
                    at_spec: "6pm".to_string()
                }
            );
        }
    }
}
