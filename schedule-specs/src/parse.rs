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
#[error("Parsing error at '{input}: {error}'")]
pub struct ParseError {
    input: String,
    error: SyntaxError,
}

impl<'a> From<NomError<'a>> for ParseError {
    fn from(error: NomError) -> Self {
        ParseError {
            input: error.0.to_owned(),
            error: error.1,
        }
    }
}

pub fn parse(time_spec: &str, day_spec: &str) -> Result<Schedule, ParseError> {
    let times = parse_time_spec(time_spec)?;
    let days = parse_day_spec(day_spec)?;
    Ok(Schedule { times, days })
}

fn parse_time_spec(spec: &str) -> Result<HashSet<WallTime>, ParseError> {
    let (_, wall_times) = terminated(time_spec, eof)(spec).finish()?;
    Ok(HashSet::from_iter(wall_times))
}

fn parse_day_spec(s: &str) -> Result<EnumSet<DayOfWeek>, ParseError> {
    todo!()
}

fn time_spec(input: &str) -> IResult<&str, Vec<WallTime>, NomError> {
    delimited(
        multispace0,
        separated_list1(segment_separator, wall_time),
        multispace0,
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

        todo!();
    }

    #[test]
    fn should_not_create_schedule_from_invalid_day_spec() {
        let result = parse("12", "nope");

        todo!();
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
        fn should_not_parse_invalid_keywird() {
            let result = parse_time_spec("11:59pm or now");

            assert_eq!(
                result.unwrap_err(),
                ParseError {
                    input: "or now".to_owned(),
                    error: SyntaxError::Nom(ErrorKind::Eof)
                }
            );
        }

        #[test]
        fn should_not_parse_out_of_range_time() {
            let result = parse_time_spec("25:69");

            assert_eq!(
                result.unwrap_err(),
                ParseError {
                    input: "25:69".to_owned(),
                    error: SyntaxError::WallTimeOutOfRange(WallTimeOutOfRange::HourOutOfRange(25))
                }
            );
        }

        #[test]
        fn should_not_parse_time_without_hours() {
            let result = parse_time_spec(":10");

            assert_eq!(
                result.unwrap_err(),
                ParseError {
                    input: ":10".to_owned(),
                    error: SyntaxError::Nom(ErrorKind::Digit)
                }
            );
        }

        #[test]
        fn should_not_parse_lone_comma() {
            let result = parse_time_spec(", and more");

            assert_eq!(
                result.unwrap_err(),
                ParseError {
                    input: ", and more".to_owned(),
                    error: SyntaxError::Nom(ErrorKind::Digit)
                }
            );
        }

        #[test]
        fn should_not_parse_lone_and() {
            let result = parse_time_spec("and");

            assert_eq!(
                result.unwrap_err(),
                ParseError {
                    input: "and".to_owned(),
                    error: SyntaxError::Nom(ErrorKind::Digit)
                }
            );
        }

        #[test]
        fn should_not_parse_lone_pm() {
            let result = parse_time_spec("pm");

            assert_eq!(
                result.unwrap_err(),
                ParseError {
                    input: "pm".to_owned(),
                    error: SyntaxError::Nom(ErrorKind::Digit)
                }
            );
        }
    }
}
