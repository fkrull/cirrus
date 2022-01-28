use super::{Hour, Minute, Repeat, Schedule, Weekday};

#[derive(thiserror::Error)]
pub enum ParseError {}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
enum At {
    Minute(Minute),
    HourAndMinute(Hour, Minute),
    DayHourMinute(Weekday, Hour, Minute),
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
enum Every {
    Hours(Repeat),
    Days(Repeat),
    Weeks(Repeat),
}

fn schedule_with_default_rate(at: At) -> Schedule {
    match at {
        At::Minute(minute) => Schedule::EveryHour(minute),
        At::HourAndMinute(hour, minute) => Schedule::EveryDay(hour, minute),
        At::DayHourMinute(weekday, hour, minute) => Schedule::EveryWeek(weekday, hour, minute),
    }
}

fn schedule(at: At, every: Every) -> Result<Schedule, ParseError> {}
