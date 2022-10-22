use super::{DayOfWeek, TimeSpec, TimeSpecOutOfRange};
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
use std::collections::BTreeSet;

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
    #[error("set of days is empty: '{0}'")]
    EmptyDaysSet(String),
}

impl ParseError {
    fn times_error(error: NomError) -> Self {
        ParseError::InvalidTimesSpec(error.0.to_owned(), SyntaxError(error.1))
    }

    fn days_error(error: NomError) -> Self {
        ParseError::InvalidDaysSpec(error.0.to_owned(), SyntaxError(error.1))
    }
}

pub fn parse_at_spec(times_string: &str) -> Result<BTreeSet<TimeSpec>, ParseError> {
    let (_, times) = terminated(time_specs, pair(multispace0, eof))(times_string)
        .finish()
        .map_err(ParseError::times_error)?;
    Ok(times)
}

pub fn parse_every_spec(days_string: &str) -> Result<EnumSet<DayOfWeek>, ParseError> {
    let (_, days) = terminated(days_spec, pair(multispace0, eof))(days_string)
        .finish()
        .map_err(ParseError::days_error)?;
    if !days.is_empty() {
        Ok(days)
    } else {
        Err(ParseError::EmptyDaysSet(days_string.to_owned()))
    }
}

fn time_specs(input: &str) -> IResult<&str, BTreeSet<TimeSpec>, NomError> {
    map(
        separated_list1(list_item_separator, time_spec),
        BTreeSet::from_iter,
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
    TimeSpec::new(hour, minute)
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
        context("'day'", value(DayOfWeek::all_days(), keyword("day"))),
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
    use super::*;
    use maplit::btreeset;
    use nom::error::ErrorKind;

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

    mod parse_at_spec {
        use super::*;

        #[test]
        fn should_parse_24h_time() {
            let result = parse_at_spec("15:23");

            assert_eq!(result.unwrap(), btreeset![TimeSpec::new(15, 23).unwrap()]);
        }

        #[test]
        fn should_parse_am_time_without_separator() {
            let result = parse_at_spec("4:39am");

            assert_eq!(result.unwrap(), btreeset![TimeSpec::new(4, 39).unwrap()]);
        }

        #[test]
        fn should_parse_am_time_with_separator() {
            let result = parse_at_spec("6:09 am");

            assert_eq!(result.unwrap(), btreeset![TimeSpec::new(6, 9).unwrap()]);
        }

        #[test]
        fn should_parse_pm_time_without_separator() {
            let result = parse_at_spec("1:7pm");

            assert_eq!(result.unwrap(), btreeset![TimeSpec::new(13, 7).unwrap()]);
        }

        #[test]
        fn should_parse_pm_time_with_separator() {
            let result = parse_at_spec("9:44 pm");

            assert_eq!(result.unwrap(), btreeset![TimeSpec::new(21, 44).unwrap()]);
        }

        #[test]
        fn should_parse_24h_time_without_minutes() {
            let result = parse_at_spec("18");

            assert_eq!(result.unwrap(), btreeset![TimeSpec::new(18, 0).unwrap()]);
        }

        #[test]
        fn should_parse_12h_time_without_minutes() {
            let result = parse_at_spec("7 pm");

            assert_eq!(result.unwrap(), btreeset![TimeSpec::new(19, 0).unwrap()]);
        }

        #[test]
        fn should_parse_multiple_times() {
            let result = parse_at_spec("1am, 2am and 6:12, and 19:59,20:00,20:01 and 11:59 pm");

            assert_eq!(
                result.unwrap(),
                btreeset![
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
            let result = parse_at_spec("    15:29   \n\t  ");

            assert_eq!(result.unwrap(), btreeset![TimeSpec::new(15, 29).unwrap()]);
        }

        #[test]
        fn should_not_parse_invalid_keyword() {
            let result = parse_at_spec("11:59pm or now");

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
            let result = parse_at_spec("25:69");

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
            let result = parse_at_spec(":10");

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
            let result = parse_at_spec(", and more");

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
            let result = parse_at_spec("and");

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
            let result = parse_at_spec("pm");

            assert_eq!(
                result.unwrap_err(),
                ParseError::InvalidTimesSpec(
                    "pm".to_owned(),
                    SyntaxError(SyntaxErrorKind::Nom(ErrorKind::Digit))
                )
            );
        }

        #[test]
        fn should_not_parse_empty_string() {
            let result = parse_at_spec("");

            assert_eq!(
                result.unwrap_err(),
                ParseError::InvalidTimesSpec(
                    "".to_owned(),
                    SyntaxError(SyntaxErrorKind::Nom(ErrorKind::Digit))
                )
            );
        }
    }

    mod parse_every_spec {
        use super::*;

        #[test]
        fn should_parse_single_day() {
            let result = parse_every_spec("  Monday   ");

            assert_eq!(result.unwrap(), DayOfWeek::Monday);
        }

        #[test]
        fn should_parse_multiple_days_with_comma() {
            let result = parse_every_spec("tuesday, wednesday");

            assert_eq!(result.unwrap(), DayOfWeek::Tuesday | DayOfWeek::Wednesday);
        }

        #[test]
        fn should_parse_multiple_days_with_and() {
            let result = parse_every_spec("Friday and Saturday");

            assert_eq!(result.unwrap(), DayOfWeek::Friday | DayOfWeek::Saturday);
        }

        #[test]
        fn should_parse_multiple_days_with_comma_and_and() {
            let result = parse_every_spec("Sunday, and Monday, and Thursday");

            assert_eq!(
                result.unwrap(),
                DayOfWeek::Sunday | DayOfWeek::Monday | DayOfWeek::Thursday
            );
        }

        #[test]
        fn should_parse_weekday_set() {
            let result = parse_every_spec("weekday");

            assert_eq!(result.unwrap(), DayOfWeek::weekdays());
        }

        #[test]
        fn should_parse_weekend_set() {
            let result = parse_every_spec("weekend");

            assert_eq!(result.unwrap(), DayOfWeek::weekend());
        }

        #[test]
        fn should_parse_set_subtracting_day() {
            let result = parse_every_spec("weekday except Wednesday");

            assert_eq!(
                result.unwrap(),
                DayOfWeek::weekdays() - DayOfWeek::Wednesday
            );
        }

        #[test]
        fn should_parse_set_subtracting_multiple_days() {
            let result = parse_every_spec("weekday except Monday, and Friday");

            assert_eq!(
                result.unwrap(),
                DayOfWeek::weekdays() - DayOfWeek::Monday - DayOfWeek::Friday
            );
        }

        #[test]
        fn should_not_parse_invalid_keyword() {
            let result = parse_every_spec("YESTERDAY");

            assert_eq!(
                result.unwrap_err(),
                ParseError::InvalidDaysSpec(
                    "YESTERDAY".to_owned(),
                    SyntaxError(SyntaxErrorKind::MultiContext(vec![
                        "list of days",
                        "'day'",
                        "'weekday'",
                        "'weekend'"
                    ]))
                )
            );
        }

        #[test]
        fn should_not_parse_digits() {
            let result = parse_every_spec("1st");

            assert_eq!(
                result.unwrap_err(),
                ParseError::InvalidDaysSpec(
                    "1st".to_owned(),
                    SyntaxError(SyntaxErrorKind::MultiContext(vec![
                        "list of days",
                        "'day'",
                        "'weekday'",
                        "'weekend'"
                    ]))
                )
            );
        }

        #[test]
        fn should_not_parse_initial_comma() {
            let result = parse_every_spec(", Monday");

            assert_eq!(
                result.unwrap_err(),
                ParseError::InvalidDaysSpec(
                    ", Monday".to_owned(),
                    SyntaxError(SyntaxErrorKind::MultiContext(vec![
                        "list of days",
                        "'day'",
                        "'weekday'",
                        "'weekend'"
                    ]))
                )
            );
        }

        #[test]
        fn should_not_parse_empty_string() {
            let result = parse_every_spec("");

            assert_eq!(
                result.unwrap_err(),
                ParseError::InvalidDaysSpec(
                    "".to_owned(),
                    SyntaxError(SyntaxErrorKind::MultiContext(vec![
                        "list of days",
                        "'day'",
                        "'weekday'",
                        "'weekend'"
                    ]))
                )
            );
        }

        #[test]
        fn should_not_empty_set_of_days() {
            let result = parse_every_spec("weekend except Saturday and Sunday");

            assert_eq!(
                result.unwrap_err(),
                ParseError::EmptyDaysSet("weekend except Saturday and Sunday".to_owned(),)
            );
        }
    }
}
