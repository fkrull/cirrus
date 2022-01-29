use std::collections::HashSet;

use enumset::EnumSet;
use nom::{
    branch::alt,
    bytes::complete::tag_no_case,
    character::complete::{char, multispace0, multispace1, u32},
    combinator::{eof, map, map_res, opt, peek, value},
    error::context,
    multi::separated_list1,
    sequence::{pair, preceded, terminated},
    Finish, IResult,
};

use super::{DayOfWeek, Schedule, TimeSpec, TimeSpecOutOfRange};

#[derive(Debug, PartialEq, Eq)]
enum SyntaxErrorKind {
    Nom(nom::error::ErrorKind),
    ExpectedChar(char),
    Context(&'static str),
    MultiContext(Vec<&'static str>),
    TimeSpecOutOfRange(TimeSpecOutOfRange),
}

impl std::fmt::Display for SyntaxErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            SyntaxErrorKind::Nom(nom::error::ErrorKind::Eof) => write!(f, "expected end-of-input"),
            SyntaxErrorKind::Nom(nom::error::ErrorKind::Digit) => write!(f, "expected digit"),
            SyntaxErrorKind::Nom(kind) => write!(f, "{:?}", kind),
            SyntaxErrorKind::ExpectedChar(c) => write!(f, "expected '{}'", c),
            SyntaxErrorKind::Context(ctx) => write!(f, "expected {}", ctx),
            SyntaxErrorKind::MultiContext(words) => {
                write!(f, "expected one of {}", words.join(", "))
            }
            SyntaxErrorKind::TimeSpecOutOfRange(e) => write!(f, "{}", e),
        }
    }
}

struct NomError<'a>(&'a str, SyntaxErrorKind);

impl<'a> nom::error::ParseError<&'a str> for NomError<'a> {
    fn from_error_kind(input: &'a str, kind: nom::error::ErrorKind) -> Self {
        NomError(input, SyntaxErrorKind::Nom(kind))
    }

    fn append(_input: &'a str, _kind: nom::error::ErrorKind, other: Self) -> Self {
        other
    }

    fn from_char(input: &'a str, c: char) -> Self {
        NomError(input, SyntaxErrorKind::ExpectedChar(c))
    }

    fn or(self, other: Self) -> Self {
        let syntax_error = match (self.1, other.1) {
            (SyntaxErrorKind::Context(word1), SyntaxErrorKind::Context(word2)) => {
                SyntaxErrorKind::MultiContext(vec![word1, word2])
            }
            (SyntaxErrorKind::MultiContext(mut words), SyntaxErrorKind::Context(word)) => {
                words.push(word);
                SyntaxErrorKind::MultiContext(words)
            }
            (SyntaxErrorKind::Context(word), SyntaxErrorKind::MultiContext(mut words)) => {
                words.insert(0, word);
                SyntaxErrorKind::MultiContext(words)
            }
            (SyntaxErrorKind::MultiContext(mut words1), SyntaxErrorKind::MultiContext(words2)) => {
                words1.extend(words2);
                SyntaxErrorKind::MultiContext(words1)
            }
            (_, other) => other,
        };
        NomError(other.0, syntax_error)
    }
}

impl<'a> nom::error::ContextError<&'a str> for NomError<'a> {
    fn add_context(input: &'a str, ctx: &'static str, _other: Self) -> Self {
        NomError(input, SyntaxErrorKind::Context(ctx))
    }
}

impl<'a> nom::error::FromExternalError<&'a str, TimeSpecOutOfRange> for NomError<'a> {
    fn from_external_error(
        input: &'a str,
        _kind: nom::error::ErrorKind,
        e: TimeSpecOutOfRange,
    ) -> Self {
        NomError(input, SyntaxErrorKind::TimeSpecOutOfRange(e))
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct SyntaxError(SyntaxErrorKind);

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum ParseError {
    #[error("invalid time specification at '{0}: {}'", (.1).0)]
    InvalidTimesSpec(String, SyntaxError),
    #[error("invalid days specification at '{0}: {}'", (.1).0)]
    InvalidDaysSpec(String, SyntaxError),
}

impl ParseError {
    fn times_error(error: NomError) -> Self {
        ParseError::InvalidTimesSpec(error.0.to_owned(), SyntaxError(error.1))
    }

    fn days_error(error: NomError) -> Self {
        ParseError::InvalidDaysSpec(error.0.to_owned(), SyntaxError(error.1))
    }
}

pub fn parse(times_string: &str, days_string: &str) -> Result<Schedule, ParseError> {
    let times = parse_times(times_string)?;
    let days = parse_days(days_string)?;
    Ok(Schedule { times, days })
}

fn parse_times(times_string: &str) -> Result<HashSet<TimeSpec>, ParseError> {
    let (_, times) = terminated(time_specs, pair(multispace0, eof))(times_string)
        .finish()
        .map_err(ParseError::times_error)?;
    Ok(times)
}

fn time_specs(input: &str) -> IResult<&str, HashSet<TimeSpec>, NomError> {
    map(
        separated_list1(list_item_separator, time_spec),
        HashSet::from_iter,
    )(input)
}

fn time_spec(input: &str) -> IResult<&str, TimeSpec, NomError> {
    map_res(
        preceded(
            multispace0,
            pair(
                pair(u32, opt(preceded(char(':'), u32))),
                opt(alt((keyword("am"), keyword("pm")))),
            ),
        ),
        to_time_spec,
    )(input)
}

fn to_time_spec(args: ((u32, Option<u32>), Option<&str>)) -> Result<TimeSpec, TimeSpecOutOfRange> {
    let ((hour, minute), suffix) = args;
    let minute = minute.unwrap_or(0);
    let is_pm = matches!(suffix, Some(s) if s.eq_ignore_ascii_case("pm"));
    let hour = hour + if is_pm { 12 } else { 0 };
    Ok(TimeSpec::new(hour, minute)?)
}

fn parse_days(days_string: &str) -> Result<EnumSet<DayOfWeek>, ParseError> {
    let (_, days) = terminated(days_spec, pair(multispace0, eof))(days_string)
        .finish()
        .map_err(ParseError::days_error)?;
    Ok(days)
}

fn days_spec(input: &str) -> IResult<&str, EnumSet<DayOfWeek>, NomError> {
    alt((list_of_days, day_group))(input)
}

fn day_group(input: &str) -> IResult<&str, EnumSet<DayOfWeek>, NomError> {
    map(
        pair(
            day_group_keyword,
            opt(preceded(keyword("except"), list_of_days)),
        ),
        map_group_of_days,
    )(input)
}

fn map_group_of_days(args: (EnumSet<DayOfWeek>, Option<EnumSet<DayOfWeek>>)) -> EnumSet<DayOfWeek> {
    match args {
        (days, Some(days_to_remove)) => days - days_to_remove,
        (days, None) => days,
    }
}

fn day_group_keyword(input: &str) -> IResult<&str, EnumSet<DayOfWeek>, NomError> {
    alt((
        context(
            "'weekday'",
            value(DayOfWeek::weekdays(), keyword("weekday")),
        ),
        context("'weekend'", value(DayOfWeek::weekend(), keyword("weekend"))),
    ))(input)
}

fn list_of_days(input: &str) -> IResult<&str, EnumSet<DayOfWeek>, NomError> {
    context(
        "list of days",
        map(
            separated_list1(list_item_separator, day_of_week),
            EnumSet::from_iter,
        ),
    )(input)
}

fn day_of_week(input: &str) -> IResult<&str, DayOfWeek, NomError> {
    alt((
        value(DayOfWeek::Monday, keyword("monday")),
        value(DayOfWeek::Tuesday, keyword("tuesday")),
        value(DayOfWeek::Wednesday, keyword("wednesday")),
        value(DayOfWeek::Thursday, keyword("thursday")),
        value(DayOfWeek::Friday, keyword("friday")),
        value(DayOfWeek::Saturday, keyword("saturday")),
        value(DayOfWeek::Sunday, keyword("sunday")),
    ))(input)
}

fn list_item_separator(input: &str) -> IResult<&str, (), NomError> {
    preceded(
        multispace0,
        alt((
            value((), pair(char(','), keyword("and"))),
            value((), char(',')),
            value((), keyword("and")),
        )),
    )(input)
}

fn keyword<'a>(
    keyword: &'static str,
) -> impl FnMut(&'a str) -> IResult<&'a str, &'a str, NomError> {
    terminated(
        preceded(multispace0, tag_no_case(keyword)),
        peek(word_separator),
    )
}

fn word_separator(input: &str) -> IResult<&str, (), NomError> {
    alt((value((), char(',')), value((), multispace1), value((), eof)))(input)
}

#[cfg(test)]
mod tests {
    use maplit::hashset;
    use nom::error::ErrorKind;

    use super::*;

    #[test]
    fn should_parse_time_spec_and_day_spec_into_schedule() {
        let result = parse("15:00 and 4:30", "weekday except monday and Friday");

        assert_eq!(
            result.unwrap(),
            Schedule {
                times: hashset![TimeSpec::new(15, 0).unwrap(), TimeSpec::new(4, 30).unwrap()],
                days: DayOfWeek::Tuesday | DayOfWeek::Wednesday | DayOfWeek::Thursday
            }
        );
    }

    #[test]
    fn should_not_create_schedule_from_invalid_time_spec() {
        let result = parse("nope", "day");

        assert_eq!(
            result.unwrap_err(),
            ParseError::InvalidTimesSpec(
                "nope".to_owned(),
                SyntaxError(SyntaxErrorKind::Nom(ErrorKind::Digit))
            )
        );
    }

    #[test]
    fn should_not_create_schedule_from_invalid_day_spec() {
        let result = parse("12", "nope");

        assert_eq!(
            result.unwrap_err(),
            ParseError::InvalidDaysSpec(
                "nope".to_owned(),
                SyntaxError(SyntaxErrorKind::MultiContext(vec![
                    "list of days",
                    "'weekday'",
                    "'weekend'"
                ]))
            )
        );
    }

    mod syntax_error {
        use super::*;

        #[test]
        fn should_format_eof() {
            let result = SyntaxErrorKind::Nom(ErrorKind::Eof).to_string();

            assert_eq!(&result, "expected end-of-input");
        }

        #[test]
        fn should_format_digits() {
            let result = SyntaxErrorKind::Nom(ErrorKind::Digit).to_string();

            assert_eq!(&result, "expected digit");
        }

        #[test]
        fn should_format_error_kind() {
            let result = SyntaxErrorKind::Nom(ErrorKind::Alpha).to_string();

            assert_eq!(&result, "Alpha");
        }

        #[test]
        fn should_format_context() {
            let result = SyntaxErrorKind::Context("time spec").to_string();

            assert_eq!(&result, "expected time spec");
        }

        #[test]
        fn should_format_multi_context() {
            let result =
                SyntaxErrorKind::MultiContext(vec!["list of days", "'weekday'"]).to_string();

            assert_eq!(&result, "expected one of list of days, 'weekday'");
        }

        #[test]
        fn should_format_expected_char() {
            let result = SyntaxErrorKind::ExpectedChar(',').to_string();

            assert_eq!(&result, "expected ','");
        }
    }

    mod parse_time_spec {
        use super::*;

        #[test]
        fn should_parse_24h_time() {
            let result = parse_times("15:23");

            assert_eq!(result.unwrap(), hashset![TimeSpec::new(15, 23).unwrap()]);
        }

        #[test]
        fn should_parse_am_time_without_separator() {
            let result = parse_times("4:39am");

            assert_eq!(result.unwrap(), hashset![TimeSpec::new(4, 39).unwrap()]);
        }

        #[test]
        fn should_parse_am_time_with_separator() {
            let result = parse_times("6:09 am");

            assert_eq!(result.unwrap(), hashset![TimeSpec::new(6, 9).unwrap()]);
        }

        #[test]
        fn should_parse_pm_time_without_separator() {
            let result = parse_times("1:7pm");

            assert_eq!(result.unwrap(), hashset![TimeSpec::new(13, 7).unwrap()]);
        }

        #[test]
        fn should_parse_pm_time_with_separator() {
            let result = parse_times("9:44 pm");

            assert_eq!(result.unwrap(), hashset![TimeSpec::new(21, 44).unwrap()]);
        }

        #[test]
        fn should_parse_24h_time_without_minutes() {
            let result = parse_times("18");

            assert_eq!(result.unwrap(), hashset![TimeSpec::new(18, 0).unwrap()]);
        }

        #[test]
        fn should_parse_12h_time_without_minutes() {
            let result = parse_times("7 pm");

            assert_eq!(result.unwrap(), hashset![TimeSpec::new(19, 0).unwrap()]);
        }

        #[test]
        fn should_parse_multiple_times() {
            let result = parse_times("1am, 2am and 6:12, and 19:59,20:00,20:01 and 11:59 pm");

            assert_eq!(
                result.unwrap(),
                hashset![
                    TimeSpec::new(1, 0).unwrap(),
                    TimeSpec::new(2, 0).unwrap(),
                    TimeSpec::new(6, 12).unwrap(),
                    TimeSpec::new(19, 59).unwrap(),
                    TimeSpec::new(20, 0).unwrap(),
                    TimeSpec::new(20, 1).unwrap(),
                    TimeSpec::new(23, 59).unwrap(),
                ]
            );
        }

        #[test]
        fn should_ignore_leading_and_trailing_whitespace() {
            let result = parse_times("    15:29   \n\t  ");

            assert_eq!(result.unwrap(), hashset![TimeSpec::new(15, 29).unwrap()]);
        }

        #[test]
        fn should_not_parse_invalid_keyword() {
            let result = parse_times("11:59pm or now");

            assert_eq!(
                result.unwrap_err(),
                ParseError::InvalidTimesSpec(
                    "or now".to_owned(),
                    SyntaxError(SyntaxErrorKind::Nom(ErrorKind::Eof))
                )
            );
        }

        #[test]
        fn should_not_parse_out_of_range_time() {
            let result = parse_times("25:69");

            assert_eq!(
                result.unwrap_err(),
                ParseError::InvalidTimesSpec(
                    "25:69".to_owned(),
                    SyntaxError(SyntaxErrorKind::TimeSpecOutOfRange(
                        TimeSpecOutOfRange::HourOutOfRange(25)
                    ))
                )
            );
        }

        #[test]
        fn should_not_parse_time_without_hours() {
            let result = parse_times(":10");

            assert_eq!(
                result.unwrap_err(),
                ParseError::InvalidTimesSpec(
                    ":10".to_owned(),
                    SyntaxError(SyntaxErrorKind::Nom(ErrorKind::Digit))
                )
            );
        }

        #[test]
        fn should_not_parse_lone_comma() {
            let result = parse_times(", and more");

            assert_eq!(
                result.unwrap_err(),
                ParseError::InvalidTimesSpec(
                    ", and more".to_owned(),
                    SyntaxError(SyntaxErrorKind::Nom(ErrorKind::Digit))
                )
            );
        }

        #[test]
        fn should_not_parse_lone_and() {
            let result = parse_times("and");

            assert_eq!(
                result.unwrap_err(),
                ParseError::InvalidTimesSpec(
                    "and".to_owned(),
                    SyntaxError(SyntaxErrorKind::Nom(ErrorKind::Digit))
                )
            );
        }

        #[test]
        fn should_not_parse_lone_pm() {
            let result = parse_times("pm");

            assert_eq!(
                result.unwrap_err(),
                ParseError::InvalidTimesSpec(
                    "pm".to_owned(),
                    SyntaxError(SyntaxErrorKind::Nom(ErrorKind::Digit))
                )
            );
        }
    }

    mod parse_day_spec {
        use super::*;

        #[test]
        fn should_parse_single_day() {
            let result = parse_days("  Monday   ");

            assert_eq!(result.unwrap(), DayOfWeek::Monday);
        }

        #[test]
        fn should_parse_multiple_days_with_comma() {
            let result = parse_days("tuesday, wednesday");

            assert_eq!(result.unwrap(), DayOfWeek::Tuesday | DayOfWeek::Wednesday);
        }

        #[test]
        fn should_parse_multiple_days_with_and() {
            let result = parse_days("Friday and Saturday");

            assert_eq!(result.unwrap(), DayOfWeek::Friday | DayOfWeek::Saturday);
        }

        #[test]
        fn should_parse_multiple_days_with_comma_and_and() {
            let result = parse_days("Sunday, and Monday, and Thursday");

            assert_eq!(
                result.unwrap(),
                DayOfWeek::Sunday | DayOfWeek::Monday | DayOfWeek::Thursday
            );
        }

        #[test]
        fn should_parse_weekday_set() {
            let result = parse_days("weekday");

            assert_eq!(result.unwrap(), DayOfWeek::weekdays());
        }

        #[test]
        fn should_parse_weekend_set() {
            let result = parse_days("weekend");

            assert_eq!(result.unwrap(), DayOfWeek::weekend());
        }

        #[test]
        fn should_parse_set_subtracting_day() {
            let result = parse_days("weekday except Wednesday");

            assert_eq!(
                result.unwrap(),
                DayOfWeek::weekdays() - DayOfWeek::Wednesday
            );
        }

        #[test]
        fn should_parse_set_subtracting_multiple_days() {
            let result = parse_days("weekday except Monday, and Friday");

            assert_eq!(
                result.unwrap(),
                DayOfWeek::weekdays() - DayOfWeek::Monday - DayOfWeek::Friday
            );
        }

        #[test]
        fn should_not_parse_invalid_keyword() {
            let result = parse_days("YESTERDAY");

            assert_eq!(
                result.unwrap_err(),
                ParseError::InvalidDaysSpec(
                    "YESTERDAY".to_owned(),
                    SyntaxError(SyntaxErrorKind::MultiContext(vec![
                        "list of days",
                        "'weekday'",
                        "'weekend'"
                    ]))
                )
            );
        }

        #[test]
        fn should_not_parse_digits() {
            let result = parse_days("1st");

            assert_eq!(
                result.unwrap_err(),
                ParseError::InvalidDaysSpec(
                    "1st".to_owned(),
                    SyntaxError(SyntaxErrorKind::MultiContext(vec![
                        "list of days",
                        "'weekday'",
                        "'weekend'"
                    ]))
                )
            );
        }

        #[test]
        fn should_not_parse_initial_comma() {
            let result = parse_days(", Monday");

            assert_eq!(
                result.unwrap_err(),
                ParseError::InvalidDaysSpec(
                    ", Monday".to_owned(),
                    SyntaxError(SyntaxErrorKind::MultiContext(vec![
                        "list of days",
                        "'weekday'",
                        "'weekend'"
                    ]))
                )
            );
        }
    }
}
