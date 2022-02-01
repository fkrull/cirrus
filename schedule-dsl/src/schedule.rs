use crate::{Schedule, TimeSpec};
use time::PrimitiveDateTime;

fn convert_day_of_week(day: time::Weekday) -> crate::DayOfWeek {
    match day {
        time::Weekday::Monday => crate::DayOfWeek::Monday,
        time::Weekday::Tuesday => crate::DayOfWeek::Tuesday,
        time::Weekday::Wednesday => crate::DayOfWeek::Wednesday,
        time::Weekday::Thursday => crate::DayOfWeek::Thursday,
        time::Weekday::Friday => crate::DayOfWeek::Friday,
        time::Weekday::Saturday => crate::DayOfWeek::Saturday,
        time::Weekday::Sunday => crate::DayOfWeek::Sunday,
    }
}

fn convert_time(t: TimeSpec) -> time::Time {
    time::Time::from_hms(t.hour as u8, t.minute as u8, 0)
        .expect("TimeSpec only contains valid hour and minute values")
}

impl Schedule {
    pub fn next_schedule(&self, after: PrimitiveDateTime) -> Option<PrimitiveDateTime> {
        self.schedule_on_same_day(after)
            .or_else(|| self.schedule_on_next_eligible_day(after))
    }

    fn schedule_on_same_day(&self, after: PrimitiveDateTime) -> Option<PrimitiveDateTime> {
        if self.days.contains(convert_day_of_week(after.weekday())) {
            self.times
                .iter()
                .copied()
                .map(convert_time)
                .find(|&t| t >= after.time())
                .map(|t| after.replace_time(t))
        } else {
            None
        }
    }

    fn schedule_on_next_eligible_day(&self, after: PrimitiveDateTime) -> Option<PrimitiveDateTime> {
        let date = std::iter::successors(after.date().next_day(), |&d| d.next_day())
            .take(7)
            .find(|d| self.days.contains(convert_day_of_week(d.weekday())))?;
        let time = self.times.iter().next()?;
        Some(PrimitiveDateTime::new(date, convert_time(*time)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DayOfWeek;
    use enumset::EnumSet;
    use maplit::btreeset;
    use time::{Date, Month, Time};

    fn dt(year: i32, month: u8, day: u8, hour: u8, minute: u8) -> PrimitiveDateTime {
        PrimitiveDateTime::new(
            Date::from_calendar_date(year, Month::try_from(month).unwrap(), day).unwrap(),
            Time::from_hms(hour, minute, 0).unwrap(),
        )
    }

    #[test]
    fn next_schedule_right_then() {
        let schedule = Schedule::from_time("15:00").unwrap();
        let time = dt(2022, 1, 31, 15, 0);
        let result = schedule.next_schedule(time);

        assert_eq!(result.unwrap(), time);
    }

    #[test]
    fn next_schedule_on_same_day() {
        let schedule = Schedule::from_time("15:00").unwrap();
        let time = dt(2022, 1, 31, 14, 30);
        let result = schedule.next_schedule(time);

        assert_eq!(result.unwrap(), dt(2022, 1, 31, 15, 0));
    }

    #[test]
    fn next_schedule_on_next_day() {
        let schedule = Schedule::from_time("15:00").unwrap();
        let time = dt(2022, 1, 31, 15, 1);
        let result = schedule.next_schedule(time);

        assert_eq!(result.unwrap(), dt(2022, 2, 1, 15, 0));
    }

    #[test]
    fn next_schedule_later_on_same_day() {
        let schedule = Schedule::from_time("14:00 and 17:00").unwrap();
        let time = dt(2022, 1, 31, 15, 1);
        let result = schedule.next_schedule(time);

        assert_eq!(result.unwrap(), dt(2022, 1, 31, 17, 0));
    }

    #[test]
    fn next_schedule_on_next_eligible_day() {
        let schedule = Schedule::from_time_and_days("16:00", "Wednesday").unwrap();
        let time = dt(2022, 1, 31, 15, 1);
        let result = schedule.next_schedule(time);

        assert_eq!(result.unwrap(), dt(2022, 2, 2, 16, 0));
    }

    #[test]
    fn next_schedule_at_earliest_time_on_next_eligible_day() {
        let schedule = Schedule::from_time_and_days("3am and 3pm", "Sunday").unwrap();
        let time = dt(2022, 1, 31, 19, 0);
        let result = schedule.next_schedule(time);

        assert_eq!(result.unwrap(), dt(2022, 2, 6, 3, 0));
    }

    #[test]
    fn next_schedule_on_earliest_eligible_day() {
        let schedule = Schedule::from_time_and_days("15:00", "Monday and Thursday").unwrap();
        let time = dt(2022, 1, 31, 15, 1);
        let result = schedule.next_schedule(time);

        assert_eq!(result.unwrap(), dt(2022, 2, 3, 15, 0));
    }

    #[test]
    fn next_schedule_almost_a_week_later() {
        let schedule = Schedule::from_time_and_days("15:00", "Monday").unwrap();
        let time = dt(2022, 1, 31, 15, 1);
        let result = schedule.next_schedule(time);

        assert_eq!(result.unwrap(), dt(2022, 2, 7, 15, 0));
    }

    #[test]
    fn should_not_find_schedule_at_the_end_of_time() {
        let schedule = Schedule::from_time("15:00").unwrap();
        let time = PrimitiveDateTime::new(Date::MAX, Time::from_hms(16, 0, 0).unwrap());
        let result = schedule.next_schedule(time);

        assert_eq!(result, None);
    }

    #[test]
    fn should_not_find_schedule_when_no_days() {
        let schedule = Schedule {
            days: EnumSet::empty(),
            times: btreeset![TimeSpec::new(17, 0).unwrap()],
            every_spec: None,
            at_spec: String::new(),
        };
        let time = dt(2022, 1, 31, 16, 0);
        let result = schedule.next_schedule(time);

        assert_eq!(result, None);
    }

    #[test]
    fn should_not_find_schedule_when_no_times() {
        let schedule = Schedule {
            days: DayOfWeek::all_days(),
            times: btreeset![],
            every_spec: None,
            at_spec: String::new(),
        };
        let time = dt(2022, 1, 31, 16, 0);
        let result = schedule.next_schedule(time);

        assert_eq!(result, None);
    }
}
