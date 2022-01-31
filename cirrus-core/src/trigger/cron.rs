use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use time::OffsetDateTime;

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum Timezone {
    Utc,
    Local,
    Other(String),
}

impl Serialize for Timezone {
    fn serialize<S>(&self, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error>
    where
        S: Serializer,
    {
        let ser = match self {
            Timezone::Utc => "utc",
            Timezone::Local => "local",
            Timezone::Other(s) => s,
        };
        serializer.serialize_str(ser)
    }
}

impl Timezone {
    fn match_tz(s: &str) -> Option<Timezone> {
        match s {
            "utc" => Some(Timezone::Utc),
            "local" => Some(Timezone::Local),
            _ => None,
        }
    }
}

struct TimezoneVisitor;

impl<'de> de::Visitor<'de> for TimezoneVisitor {
    type Value = Timezone;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, r#""utc", "local", or the name of a time zone"#)
    }

    fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let tz = Timezone::match_tz(s).unwrap_or_else(|| Timezone::Other(s.to_string()));
        Ok(tz)
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let tz = Timezone::match_tz(&v).unwrap_or_else(|| Timezone::Other(v));
        Ok(tz)
    }
}

impl<'de> Deserialize<'de> for Timezone {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_string(TimezoneVisitor)
    }
}

impl Default for Timezone {
    fn default() -> Self {
        Timezone::Local
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
pub struct Cron {
    pub(crate) cron: String,
    #[serde(default)]
    pub(crate) timezone: Timezone,
}

fn time_to_chrono(time: OffsetDateTime) -> chrono::DateTime<chrono::Utc> {
    chrono::DateTime::from_utc(
        chrono::NaiveDateTime::new(
            chrono::NaiveDate::from_ymd(
                time.year(),
                u8::from(time.month()) as u32,
                time.day() as u32,
            ),
            chrono::NaiveTime::from_hms_nano(
                time.hour() as u32,
                time.minute() as u32,
                time.second() as u32,
                time.nanosecond(),
            ),
        ),
        chrono::Utc,
    )
}

fn chrono_to_time(chrono: chrono::DateTime<chrono::Utc>) -> OffsetDateTime {
    use chrono::{Datelike, Timelike};
    use std::convert::TryFrom;

    time::Date::from_calendar_date(
        chrono.year(),
        time::Month::try_from(chrono.month() as u8)
            .expect("chrono datetime contains only valid months"),
        chrono.day() as u8,
    )
    .expect("chrono datetime contains only valid dates")
    .with_hms_nano(
        chrono.hour() as u8,
        chrono.minute() as u8,
        chrono.second() as u8,
        chrono.nanosecond(),
    )
    .expect("chrono datetime contains only valid times")
    .assume_offset(time::UtcOffset::UTC)
}

impl Cron {
    pub fn next_schedule(&self, after: time::OffsetDateTime) -> eyre::Result<time::OffsetDateTime> {
        let after = time_to_chrono(after.to_offset(time::UtcOffset::UTC));
        let next = match self {
            Cron {
                cron,
                timezone: Timezone::Utc,
            } => cron_parser::parse(cron, &after)?,
            Cron {
                cron,
                timezone: Timezone::Local,
            } => cron_parser::parse(cron, &after.with_timezone(&chrono::Local))?
                .with_timezone(&chrono::Utc),
            Cron {
                timezone: Timezone::Other(_),
                ..
            } => return Err(eyre::eyre!("arbitrary timezones aren't supported")),
        };
        Ok(chrono_to_time(next))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::format_description::well_known::Rfc3339;

    mod timezone {
        use super::*;

        #[test]
        fn should_deserialize_utc_timezone() {
            let tz: Timezone = serde_json::from_str(r#""utc""#).unwrap();
            assert_eq!(tz, Timezone::Utc);
        }

        #[test]
        fn should_deserialize_local_timezone() {
            let tz: Timezone = serde_json::from_str(r#""local""#).unwrap();
            assert_eq!(tz, Timezone::Local);
        }

        #[test]
        fn should_deserialize_other_timezone() {
            let tz: Timezone = serde_json::from_str(r#""Antarctica/Troll""#).unwrap();
            assert_eq!(tz, Timezone::Other("Antarctica/Troll".to_string()));
        }

        #[test]
        fn should_serialize_utc_timezone() {
            let s = serde_json::to_string(&Timezone::Utc).unwrap();
            assert_eq!(&s, r#""utc""#);
        }

        #[test]
        fn should_serialize_local_timezone() {
            let s = serde_json::to_string(&Timezone::Local).unwrap();
            assert_eq!(&s, r#""local""#);
        }

        #[test]
        fn should_serialize_other_timezone() {
            let s =
                serde_json::to_string(&Timezone::Other("Africa/Casablanca".to_string())).unwrap();
            assert_eq!(&s, r#""Africa/Casablanca""#);
        }
    }

    #[test]
    fn should_get_next_schedule_for_cron_expression() {
        let trigger = Cron {
            cron: "30 10 * * *".to_string(),
            timezone: Timezone::Utc,
        };
        let next = trigger
            .next_schedule(OffsetDateTime::parse("2020-05-14T09:56:13.123Z", &Rfc3339).unwrap())
            .unwrap();
        assert_eq!(
            next,
            OffsetDateTime::parse("2020-05-14T10:30:00Z", &Rfc3339).unwrap()
        );
    }

    #[test]
    fn should_get_next_schedule_for_another_cron_expression() {
        let trigger = Cron {
            cron: "0 */6 * * *".to_string(),
            timezone: Timezone::Utc,
        };
        let next = trigger
            .next_schedule(OffsetDateTime::parse("2020-05-15T00:04:52.123Z", &Rfc3339).unwrap())
            .unwrap();
        assert_eq!(
            next,
            OffsetDateTime::parse("2020-05-15T06:00:00Z", &Rfc3339).unwrap()
        );
    }

    // TODO: fix local time
    /*#[test]
    fn should_get_next_schedule_for_a_cron_expression_using_local_time() {
        let trigger = Cron {
            cron: "34 13 15 5 *".to_string(),
            timezone: Timezone::Local,
        };
        let local = PrimitiveDateTime::parse("2020-04-16T07:13:31.666", &Rfc3339).unwrap().assume_offset(UtcOffset::current_local_offset())
        let local = chrono::Local
            .from_local_datetime(&NaiveDateTime::from_str().unwrap())
            .unwrap();
        let expected_local = chrono::Local
            .from_local_datetime(&NaiveDateTime::from_str("2020-05-15T13:34:00").unwrap())
            .unwrap();

        let next = trigger
            .next_schedule(local.with_timezone(&chrono::Utc))
            .unwrap();

        assert_eq!(next, expected_local.with_timezone(&chrono::Utc));
    }*/
}
