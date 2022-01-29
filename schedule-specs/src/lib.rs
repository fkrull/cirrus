use enumset::EnumSet;
use std::collections::HashSet;

pub mod parse;

#[derive(Debug, Hash, enumset::EnumSetType)]
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
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum TimeSpecOutOfRange {
    #[error("hour value {0} out of range [0,23]")]
    HourOutOfRange(u32),
    #[error("minute value {0} out of range [0,59]")]
    MinuteOutOfRange(u32),
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Schedule {
    pub days: EnumSet<DayOfWeek>,
    pub times: HashSet<TimeSpec>,
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
}
