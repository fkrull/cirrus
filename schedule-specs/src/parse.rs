use super::{DayOfWeek, Schedule, WallTime};
use crate::WallTimeOutOfRange;
use enumset::EnumSet;
use nom::bytes::complete::tag_no_case;
use nom::character::complete::{digit1, u32};
use nom::combinator::{map_res, opt};
use nom::error::context;
use nom::multi::separated_list1;
use nom::sequence::{pair, preceded, separated_pair};
use nom::{
    branch::alt,
    bytes::complete::is_a,
    character::complete::{alpha1, char, multispace0, multispace1},
    combinator::{eof, map, peek, value},
    sequence::{delimited, terminated},
    Finish, IResult, Parser,
};
use std::collections::HashSet;

#[derive(Debug)]
enum SyntaxError {
    Nom(nom::error::ErrorKind),
    ExpectedChar(char),
    Context(&'static str),
    WallTimeOutOfRange(WallTimeOutOfRange),
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

#[derive(Debug, thiserror::Error)]
#[error("halp")]
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

    mod parse_time_spec {
        use super::*;

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
        fn should_not_parse_invalid_spec() {
            let result = parse_time_spec("11:59pm or now");

            assert!(matches!(dbg!(result), Err(_)));
        }

        #[test]
        fn should_not_parse_out_of_range_time() {
            let result = parse_time_spec("25:69");

            assert!(matches!(dbg!(result), Err(_)));
        }

        #[test]
        fn should_not_parse_invalid_numbers() {
            let result = parse_time_spec("1e5");

            assert!(matches!(dbg!(result), Err(_)));
        }
    }
}
