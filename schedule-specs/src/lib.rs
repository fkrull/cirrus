use enumset::{EnumSet, EnumSetType};
use std::collections::HashSet;

mod parse;

#[derive(Debug, Hash, EnumSetType)]
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
pub enum WallTimeOutOfRange {
    #[error("hour value {0} out of range [0,23]")]
    HourOutOfRange(u32),
    #[error("minute value {0} out of range [0,59]")]
    MinuteOutOfRange(u32),
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct WallTime {
    hour: u32,
    minute: u32,
}

impl WallTime {
    pub fn new(hour: u32, minute: u32) -> Result<WallTime, WallTimeOutOfRange> {
        if hour > 23 {
            Err(WallTimeOutOfRange::HourOutOfRange(hour))
        } else if minute > 59 {
            Err(WallTimeOutOfRange::MinuteOutOfRange(minute))
        } else {
            Ok(WallTime { hour, minute })
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
    days: EnumSet<DayOfWeek>,
    times: HashSet<WallTime>,
}

#[cfg(test)]
mod tests {
    use super::*;

    mod walltime {
        use super::*;

        #[test]
        fn should_create_zero_walltime() {
            let result = WallTime::new(0, 0);

            assert_eq!(result.unwrap(), WallTime { hour: 0, minute: 0 });
        }

        #[test]
        fn should_create_max_walltime() {
            let result = WallTime::new(23, 59);

            assert_eq!(
                result.unwrap(),
                WallTime {
                    hour: 23,
                    minute: 59
                }
            );
        }

        #[test]
        fn should_check_hour_range() {
            let result = WallTime::new(24, 30);

            assert!(matches!(
                result,
                Err(WallTimeOutOfRange::HourOutOfRange(24))
            ));
        }

        #[test]
        fn should_check_minute_range() {
            let result = WallTime::new(12, 60);

            assert!(matches!(
                result,
                Err(WallTimeOutOfRange::MinuteOutOfRange(60))
            ));
        }
    }
}
