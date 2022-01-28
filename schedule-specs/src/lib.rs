pub mod parse;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct Hour(pub u32);

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct Minute(pub u32);

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum Weekday {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct Repeat(pub u32);

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum Anchor {}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum Schedule {
    EveryHour(Minute),
    EveryFewHours(Hour, Minute, Repeat),
    EveryDay(Hour, Minute),
    EveryFewDays(Weekday, Hour, Minute, Repeat),
    EveryWeek(Weekday, Hour, Minute),
    EveryFewWeeks(Weekday, Hour, Minute, Repeat),
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
