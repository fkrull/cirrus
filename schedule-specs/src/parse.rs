use super::{DayOfWeek, Schedule, WallTime};
use enumset::EnumSet;
use nom::branch::alt;
use nom::bytes::complete::is_a;
use nom::character::complete::{alpha1, char, digit1, multispace1};
use nom::character::streaming::multispace0;
use nom::combinator::{map, peek, value};
use nom::sequence::{delimited, pair, preceded, terminated};
use nom::IResult;
use std::collections::HashSet;

#[derive(Debug, thiserror::Error)]
pub enum ParseError {}

pub fn parse(time_spec: &str, day_spec: &str) -> Result<Schedule, ParseError> {
    let times = parse_time_spec(time_spec)?;
    let days = parse_day_spec(day_spec)?;
    Ok(Schedule { times, days })
}

fn parse_time_spec(s: &str) -> Result<HashSet<WallTime>, ParseError> {
    todo!()
}

fn parse_day_spec(s: &str) -> Result<EnumSet<DayOfWeek>, ParseError> {
    todo!()
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum Token<'a> {
    Word(&'a str),
    TimeString(&'a str),
    Comma,
}

fn token(input: &str) -> IResult<&str, Token> {
    delimited(multispace0, alt((word, time_string, comma)), multispace0)(input)
}

fn word(input: &str) -> IResult<&str, Token> {
    map(terminated(alpha1, peek(word_separator)), Token::Word)(input)
}

fn word_separator(input: &str) -> IResult<&str, ()> {
    alt((value((), char(',')), value((), multispace1)))(input)
}

fn time_string(input: &str) -> IResult<&str, Token> {
    map(is_a("1234567890:"), Token::TimeString)(input)
}

fn comma(input: &str) -> IResult<&str, Token> {
    value(Token::Comma, char(','))(input)
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
    }

    mod token {
        use super::*;

        #[test]
        fn should_tokenize_word() {
            let result = token("  word rest");

            assert_eq!(result.unwrap(), ("rest", Token::Word("word")));
        }

        #[test]
        fn should_tokenize_time_string() {
            let result = token("  15:32:32:1:000:1 rest");

            assert_eq!(
                result.unwrap(),
                ("rest", Token::TimeString("15:32:32:1:000:1"))
            );
        }

        #[test]
        fn should_tokenize_single_comma() {
            let result = token("  ,,");

            assert_eq!(result.unwrap(), (",", Token::Comma));
        }

        #[test]
        fn should_tokenize_word_terminated_by_comma() {
            let result = token("abc,def");

            assert_eq!(result.unwrap(), (",def", Token::Word("abc")));
        }

        #[test]
        fn should_tokenize_time_string_terminated_by_word() {
            let result = token("5:30pm");

            assert_eq!(result.unwrap(), ("pm", Token::TimeString("5:30")));
        }
    }
}
