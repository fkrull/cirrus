use super::{DayOfWeek, Schedule, WallTime};
use crate::WallTimeOutOfRange;
use enumset::EnumSet;
use nom::bytes::complete::tag_no_case;
use nom::character::complete::digit1;
use nom::combinator::{map_res, opt};
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
pub struct TokenizeError(nom::error::ErrorKind);

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("error tokenizing '{0}': {}", (.1).0.description())]
    TokenizeError(String, TokenizeError),
    #[error("invalid number '{0}'")]
    InvalidNumber(String),
    #[error(transparent)]
    WalltimeOutOfRange(#[from] WallTimeOutOfRange),
    #[error("parser error at '{0}': {}", (.1).description())]
    ParseError(String, nom::error::ErrorKind),
}

pub fn parse(time_spec: &str, day_spec: &str) -> Result<Schedule, ParseError> {
    let times = parse_time_spec(time_spec)?;
    let days = parse_day_spec(day_spec)?;
    Ok(Schedule { times, days })
}

fn parse_time_spec(s: &str) -> Result<HashSet<WallTime>, ParseError> {
    match single_time_spec(s).finish() {
        Ok((x, y)) => {
            let mut s = HashSet::new();
            s.insert(y);
            Ok(s)
        }
        Err(error) => Err(ParseError::ParseError(error.input.to_owned(), error.code)),
    }
}

fn parse_day_spec(s: &str) -> Result<EnumSet<DayOfWeek>, ParseError> {
    todo!()
}

fn single_time_spec(input: &str) -> IResult<&str, WallTime> {
    map_res(
        preceded(
            multispace0,
            pair(
                pair(digit1, opt(preceded(char(':'), digit1))),
                opt(alt((keyword("am"), keyword("pm")))),
            ),
        ),
        to_wall_time,
    )(input)
}

fn to_wall_time(args: ((&str, Option<&str>), Option<&str>)) -> Result<WallTime, ParseError> {
    let ((hour, minute), suffix) = args;
    let hour = hour
        .parse::<u32>()
        .map_err(|_| ParseError::InvalidNumber(hour.to_owned()))?;
    let minute = match minute {
        Some(minute) => minute
            .parse::<u32>()
            .map_err(|_| ParseError::InvalidNumber(minute.to_owned()))?,
        None => 0,
    };
    let hour = hour
        + match suffix {
            Some(x) if x.eq_ignore_ascii_case("pm") => 12,
            _ => 0,
        };
    Ok(WallTime::new(hour, minute)?)
}

fn keyword<'b, 'a: 'b>(keyword: &'b str) -> impl FnMut(&'a str) -> IResult<&'a str, &'a str> + 'b {
    let x = terminated(
        preceded(multispace0, tag_no_case(keyword)),
        peek(word_separator),
    );
    x
}

fn ws_before<'a, O>(
    p: impl FnMut(&'a str) -> IResult<&'a str, O>,
) -> impl FnMut(&'a str) -> IResult<&'a str, O> {
    preceded(multispace0, p)
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum Token<'a> {
    Word(&'a str),
    TimeString(&'a str),
    Comma,
    None,
}

fn tokenize<'a>(input: &'a str, tokens: &mut [Token<'a>]) -> Result<&'a str, ParseError> {
    let mut input = input;
    for idx in 0..tokens.len() {
        if input.is_empty() {
            tokens[idx] = Token::None;
            break;
        }
        match token(input).finish() {
            Ok((remaining, token)) => {
                input = remaining;
                tokens[idx] = token;
            }
            Err(error) => {
                return Err(ParseError::TokenizeError(
                    error.input.to_owned(),
                    TokenizeError(error.code),
                ));
            }
        }
    }

    Ok(input)
}

fn token(input: &str) -> IResult<&str, Token> {
    delimited(multispace0, alt((word, time_string, comma)), multispace0)(input)
}

fn word(input: &str) -> IResult<&str, Token> {
    map(terminated(alpha1, peek(word_separator)), Token::Word)(input)
}

fn word_separator(input: &str) -> IResult<&str, ()> {
    alt((value((), char(',')), value((), multispace1), value((), eof)))(input)
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

    mod tokenize {
        use super::*;

        #[test]
        fn should_tokenize() {
            let mut tokens = [Token::None; 6];
            let result = tokenize("  at 12:33pm, and noon", &mut tokens);

            assert_eq!(result.unwrap(), "");
            assert_eq!(
                &tokens,
                &[
                    Token::Word("at"),
                    Token::TimeString("12:33"),
                    Token::Word("pm"),
                    Token::Comma,
                    Token::Word("and"),
                    Token::Word("noon")
                ]
            );
        }

        #[test]
        fn should_tokenize_missing() {
            let mut tokens = [Token::Comma; 3];
            let result = tokenize("word", &mut tokens);

            assert_eq!(result.unwrap(), "");
            assert_eq!(&tokens, &[Token::Word("word"), Token::None, Token::Comma,]);
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
