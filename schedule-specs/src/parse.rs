use super::{DayOfWeek, Schedule, WallTime, WallTimeOutOfRange};
use enumset::EnumSet;
use nom::{
    branch::alt,
    bytes::complete::{is_a, tag_no_case},
    character::complete::{alpha1, char, digit1, multispace0, multispace1, u32},
    combinator::{eof, map, map_res, opt, peek, value},
    error::context,
    multi::separated_list1,
    sequence::{delimited, pair, preceded, separated_pair, terminated},
    Finish, IResult, Parser,
};
use std::collections::HashSet;

#[derive(Debug, PartialEq, Eq)]
enum SyntaxError {
    Nom(nom::error::ErrorKind),
    ExpectedChar(char),
    Context(&'static str),
    WallTimeOutOfRange(WallTimeOutOfRange),
}

impl std::fmt::Display for SyntaxError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            SyntaxError::Nom(nom::error::ErrorKind::Eof) => write!(f, "expected end-of-input"),
            SyntaxError::Nom(nom::error::ErrorKind::Digit) => write!(f, "expected digit"),
            SyntaxError::Nom(kind) => write!(f, "{:?}", kind),
            SyntaxError::ExpectedChar(c) => write!(f, "expected '{}'", c),
            SyntaxError::Context(ctx) => write!(f, "expected keyword '{}'", ctx),
            SyntaxError::WallTimeOutOfRange(e) => write!(f, "{}", e),
        }
    }
}

struct NomError<'a>(&'a str, SyntaxError);

impl<'a> nom::error::ParseError<&'a str> for NomError<'a> {
    fn from_error_kind(input: &'a str, kind: nom::error::ErrorKind) -> Self {
        NomError(input, SyntaxError::Nom(kind))
    }

    fn append(_input: &'a str, _kind: nom::error::ErrorKind, other: Self) -> Self {
        other
    }

    fn from_char(input: &'a str, c: char) -> Self {
        NomError(input, SyntaxError::ExpectedChar(c))
    }
}

impl<'a> nom::error::ContextError<&'a str> for NomError<'a> {
    fn add_context(input: &'a str, ctx: &'static str, _other: Self) -> Self {
        NomError(input, SyntaxError::Context(ctx))
    }
}

impl<'a> nom::error::FromExternalError<&'a str, WallTimeOutOfRange> for NomError<'a> {
    fn from_external_error(
        input: &'a str,
        _kind: nom::error::ErrorKind,
        e: WallTimeOutOfRange,
    ) -> Self {
        NomError(input, SyntaxError::WallTimeOutOfRange(e))
    }
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum ParseError {
    #[error("invalid time specification at '{0}: {1}'")]
    InvalidTimeSpec(String, SyntaxError),
    #[error("invalid days specification at '{0}: {1}'")]
    InvalidDaySpec(String, SyntaxError),
}

impl ParseError {
    fn from_time_spec_error(error: NomError) -> Self {
        ParseError::InvalidTimeSpec(error.0.to_owned(), error.1)
    }

    fn from_day_spec_error(error: NomError) -> Self {
        ParseError::InvalidDaySpec(error.0.to_owned(), error.1)
    }
}

pub fn parse(time_spec: &str, day_spec: &str) -> Result<Schedule, ParseError> {
    let times = parse_time_spec(time_spec)?;
    let days = parse_day_spec(day_spec)?;
    Ok(Schedule { times, days })
}

fn parse_time_spec(spec: &str) -> Result<HashSet<WallTime>, ParseError> {
    let (_, wall_times) = terminated(time_spec, pair(multispace0, eof))(spec)
        .finish()
        .map_err(ParseError::from_time_spec_error)?;
    Ok(wall_times)
}

fn time_spec(input: &str) -> IResult<&str, HashSet<WallTime>, NomError> {
    map(
        preceded(multispace0, separated_list1(segment_separator, wall_time)),
        HashSet::from_iter,
    )(input)
}

fn wall_time(input: &str) -> IResult<&str, WallTime, NomError> {
    map_res(
        preceded(
            multispace0,
            pair(
                pair(u32, opt(preceded(char(':'), u32))),
                opt(alt((keyword("am"), keyword("pm")))),
            ),
        ),
        to_wall_time,
    )(input)
}

fn to_wall_time(args: ((u32, Option<u32>), Option<&str>)) -> Result<WallTime, WallTimeOutOfRange> {
    let ((hour, minute), suffix) = args;
    let minute = minute.unwrap_or(0);
    let is_pm = matches!(suffix, Some(s) if s.eq_ignore_ascii_case("pm"));
    let hour = hour + if is_pm { 12 } else { 0 };
    Ok(WallTime::new(hour, minute)?)
}

fn parse_day_spec(spec: &str) -> Result<EnumSet<DayOfWeek>, ParseError> {
    let (_, days) = terminated(day_spec, pair(multispace0, eof))(spec)
        .finish()
        .map_err(ParseError::from_day_spec_error)?;
    Ok(days)
}

fn day_spec(input: &str) -> IResult<&str, EnumSet<DayOfWeek>, NomError> {
    alt((days_of_week, day_set_expression))(input)
}

fn day_set_expression(input: &str) -> IResult<&str, EnumSet<DayOfWeek>, NomError> {
    map(
        pair(day_set, opt(preceded(keyword("except"), days_of_week))),
        map_day_set,
    )(input)
}

fn map_day_set(args: (EnumSet<DayOfWeek>, Option<EnumSet<DayOfWeek>>)) -> EnumSet<DayOfWeek> {
    match args {
        (day_set, Some(days_to_remove)) => day_set - days_to_remove,
        (day_set, None) => day_set,
    }
}

fn day_set(input: &str) -> IResult<&str, EnumSet<DayOfWeek>, NomError> {
    alt((
        keyword_with_value("weekday", DayOfWeek::weekdays()),
        keyword_with_value("weekend", DayOfWeek::weekend()),
    ))(input)
}

fn days_of_week(input: &str) -> IResult<&str, EnumSet<DayOfWeek>, NomError> {
    map(
        preceded(multispace0, separated_list1(segment_separator, day_of_week)),
        |days| days.into_iter().collect(),
    )(input)
}

fn day_of_week(input: &str) -> IResult<&str, DayOfWeek, NomError> {
    alt((
        keyword_with_value("monday", DayOfWeek::Monday),
        keyword_with_value("tuesday", DayOfWeek::Tuesday),
        keyword_with_value("wednesday", DayOfWeek::Wednesday),
        keyword_with_value("thursday", DayOfWeek::Thursday),
        keyword_with_value("friday", DayOfWeek::Friday),
        keyword_with_value("saturday", DayOfWeek::Saturday),
        keyword_with_value("sunday", DayOfWeek::Sunday),
    ))(input)
}

fn keyword_with_value<'a, T: Clone>(
    k: &'static str,
    v: T,
) -> impl FnMut(&'a str) -> IResult<&'a str, T, NomError> {
    value(v, keyword(k))
}

fn segment_separator(input: &str) -> IResult<&str, (), NomError> {
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
    context(
        keyword,
        terminated(
            preceded(multispace0, tag_no_case(keyword)),
            peek(word_separator),
        ),
    )
}

fn word_separator(input: &str) -> IResult<&str, (), NomError> {
    alt((value((), char(',')), value((), multispace1), value((), eof)))(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use maplit::hashset;
    use nom::error::ErrorKind;

    #[test]
    fn should_parse_time_spec_and_day_spec_into_schedule() {
        let result = parse("15:00 and 4:30", "weekday except monday and Friday");

        assert_eq!(
            result.unwrap(),
            Schedule {
                times: hashset![WallTime::new(15, 0).unwrap(), WallTime::new(4, 30).unwrap()],
                days: DayOfWeek::Tuesday | DayOfWeek::Wednesday | DayOfWeek::Thursday
            }
        );
    }

    #[test]
    fn should_not_create_schedule_from_invalid_time_spec() {
        let result = parse("nope", "day");

        assert_eq!(
            result.unwrap_err(),
            ParseError::InvalidTimeSpec("nope".to_owned(), SyntaxError::Nom(ErrorKind::Digit))
        );
    }

    #[test]
    fn should_not_create_schedule_from_invalid_day_spec() {
        let result = parse("12", "nope");

        assert_eq!(
            result.unwrap_err(),
            ParseError::InvalidDaySpec("nope".to_owned(), SyntaxError::Nom(ErrorKind::Digit))
        );
    }

    mod syntax_error {
        use super::*;
        use nom::error::ErrorKind;

        #[test]
        fn should_format_eof() {
            let result = SyntaxError::Nom(ErrorKind::Eof).to_string();

            assert_eq!(&result, "expected end-of-input");
        }

        #[test]
        fn should_format_digits() {
            let result = SyntaxError::Nom(ErrorKind::Digit).to_string();

            assert_eq!(&result, "expected digit");
        }

        #[test]
        fn should_format_error_kind() {
            let result = SyntaxError::Nom(ErrorKind::Alpha).to_string();

            assert_eq!(&result, "Alpha");
        }

        #[test]
        fn should_format_ctx() {
            let result = SyntaxError::Context("monday").to_string();

            assert_eq!(&result, "expected keyword 'monday'");
        }

        #[test]
        fn should_format_expected_char() {
            let result = SyntaxError::ExpectedChar(',').to_string();

            assert_eq!(&result, "expected ','");
        }
    }

    mod parse_time_spec {
        use super::*;
        use nom::error::ErrorKind;

        #[test]
        fn should_parse_24h_time() {
            let result = parse_time_spec("15:23");

            assert_eq!(result.unwrap(), hashset![WallTime::new(15, 23).unwrap()]);
        }

        #[test]
        fn should_parse_am_time_without_separator() {
            let result = parse_time_spec("4:39am");

            assert_eq!(result.unwrap(), hashset![WallTime::new(4, 39).unwrap()]);
        }

        #[test]
        fn should_parse_am_time_with_separator() {
            let result = parse_time_spec("6:09 am");

            assert_eq!(result.unwrap(), hashset![WallTime::new(6, 9).unwrap()]);
        }

        #[test]
        fn should_parse_pm_time_without_separator() {
            let result = parse_time_spec("1:7pm");

            assert_eq!(result.unwrap(), hashset![WallTime::new(13, 7).unwrap()]);
        }

        #[test]
        fn should_parse_pm_time_with_separator() {
            let result = parse_time_spec("9:44 pm");

            assert_eq!(result.unwrap(), hashset![WallTime::new(21, 44).unwrap()]);
        }

        #[test]
        fn should_parse_24h_time_without_minutes() {
            let result = parse_time_spec("18");

            assert_eq!(result.unwrap(), hashset![WallTime::new(18, 0).unwrap()]);
        }

        #[test]
        fn should_parse_12h_time_without_minutes() {
            let result = parse_time_spec("7 pm");

            assert_eq!(result.unwrap(), hashset![WallTime::new(19, 0).unwrap()]);
        }

        #[test]
        fn should_parse_multiple_times() {
            let result = parse_time_spec("1am, 2am and 6:12, and 19:59,20:00,20:01 and 11:59 pm");

            assert_eq!(
                result.unwrap(),
                hashset![
                    WallTime::new(1, 0).unwrap(),
                    WallTime::new(2, 0).unwrap(),
                    WallTime::new(6, 12).unwrap(),
                    WallTime::new(19, 59).unwrap(),
                    WallTime::new(20, 0).unwrap(),
                    WallTime::new(20, 1).unwrap(),
                    WallTime::new(23, 59).unwrap(),
                ]
            );
        }

        #[test]
        fn should_ignore_leading_and_trailing_whitespace() {
            let result = parse_time_spec("    15:29   \n\t  ");

            assert_eq!(result.unwrap(), hashset![WallTime::new(15, 29).unwrap()]);
        }

        #[test]
        fn should_not_parse_invalid_keyword() {
            let result = parse_time_spec("11:59pm or now");

            assert_eq!(
                result.unwrap_err(),
                ParseError::InvalidTimeSpec("or now".to_owned(), SyntaxError::Nom(ErrorKind::Eof))
            );
        }

        #[test]
        fn should_not_parse_out_of_range_time() {
            let result = parse_time_spec("25:69");

            assert_eq!(
                result.unwrap_err(),
                ParseError::InvalidTimeSpec(
                    "25:69".to_owned(),
                    SyntaxError::WallTimeOutOfRange(WallTimeOutOfRange::HourOutOfRange(25))
                )
            );
        }

        #[test]
        fn should_not_parse_time_without_hours() {
            let result = parse_time_spec(":10");

            assert_eq!(
                result.unwrap_err(),
                ParseError::InvalidTimeSpec(":10".to_owned(), SyntaxError::Nom(ErrorKind::Digit))
            );
        }

        #[test]
        fn should_not_parse_lone_comma() {
            let result = parse_time_spec(", and more");

            assert_eq!(
                result.unwrap_err(),
                ParseError::InvalidTimeSpec(
                    ", and more".to_owned(),
                    SyntaxError::Nom(ErrorKind::Digit)
                )
            );
        }

        #[test]
        fn should_not_parse_lone_and() {
            let result = parse_time_spec("and");

            assert_eq!(
                result.unwrap_err(),
                ParseError::InvalidTimeSpec("and".to_owned(), SyntaxError::Nom(ErrorKind::Digit))
            );
        }

        #[test]
        fn should_not_parse_lone_pm() {
            let result = parse_time_spec("pm");

            assert_eq!(
                result.unwrap_err(),
                ParseError::InvalidTimeSpec("pm".to_owned(), SyntaxError::Nom(ErrorKind::Digit))
            );
        }
    }

    mod parse_day_spec {
        use super::*;
        use nom::error::ErrorKind;

        #[test]
        fn should_parse_single_day() {
            let result = parse_day_spec("  Monday   ");

            assert_eq!(result.unwrap(), DayOfWeek::Monday);
        }

        #[test]
        fn should_parse_multiple_days_with_comma() {
            let result = parse_day_spec("tuesday, wednesday");

            assert_eq!(result.unwrap(), DayOfWeek::Tuesday | DayOfWeek::Wednesday);
        }

        #[test]
        fn should_parse_multiple_days_with_and() {
            let result = parse_day_spec("Friday and Saturday");

            assert_eq!(result.unwrap(), DayOfWeek::Friday | DayOfWeek::Saturday);
        }

        #[test]
        fn should_parse_multiple_days_with_comma_and_and() {
            let result = parse_day_spec("Sunday, and Monday, and Thursday");

            assert_eq!(
                result.unwrap(),
                DayOfWeek::Sunday | DayOfWeek::Monday | DayOfWeek::Thursday
            );
        }

        #[test]
        fn should_parse_weekday_set() {
            let result = parse_day_spec("weekday");

            assert_eq!(result.unwrap(), DayOfWeek::weekdays());
        }

        #[test]
        fn should_parse_weekend_set() {
            let result = parse_day_spec("weekend");

            assert_eq!(result.unwrap(), DayOfWeek::weekend());
        }

        #[test]
        fn should_parse_set_subtracting_day() {
            let result = parse_day_spec("weekday except Wednesday");

            assert_eq!(
                result.unwrap(),
                DayOfWeek::weekdays() - DayOfWeek::Wednesday
            );
        }

        #[test]
        fn should_parse_set_subtracting_multiple_days() {
            let result = parse_day_spec("weekday except Monday, and Friday");

            assert_eq!(
                result.unwrap(),
                DayOfWeek::weekdays() - DayOfWeek::Monday - DayOfWeek::Friday
            );
        }

        #[test]
        fn should_not_parse_invalid_keyword() {
            let result = parse_day_spec("YESTERDAY");

            assert_eq!(
                result.unwrap_err(),
                ParseError::InvalidDaySpec(
                    "YESTERDAY".to_owned(),
                    SyntaxError::Nom(ErrorKind::Eof)
                )
            );
        }

        #[test]
        fn should_not_parse_digits() {
            let result = parse_day_spec("1st");

            assert_eq!(
                result.unwrap_err(),
                ParseError::InvalidDaySpec("1st".to_owned(), SyntaxError::Nom(ErrorKind::Eof))
            );
        }

        #[test]
        fn should_not_parse_initial_comma() {
            let result = parse_day_spec(", Monday");

            assert_eq!(
                result.unwrap_err(),
                ParseError::InvalidDaySpec(", Monday".to_owned(), SyntaxError::Nom(ErrorKind::Eof))
            );
        }
    }
}
