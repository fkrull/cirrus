/// Get the current build version.
#[derive(argh::FromArgs)]
struct Args {}

fn main() -> eyre::Result<()> {
    let changelog = std::fs::read_to_string("CHANGELOG.md")?;
    let version = find_release_version(changelog.lines())
        .ok_or_else(|| eyre::eyre!("failed to find release version in changelog file"))?;
    let build_date = BuildDate::now();
    println!("VERSION={}", version);
    println!("BUILD_NBUMBER={}", build_date.build_string());
    Ok(())
}

fn find_release_version<'a>(lines: impl Iterator<Item = &'a str>) -> Option<&'a str> {
    lines.filter_map(find_in_line).next()
}

fn find_in_line(line: &str) -> Option<&str> {
    line.trim()
        .strip_prefix("## ")
        .and_then(|s| s.split("-").next())
        .map(|p| p.trim())
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
struct BuildDate {
    year: u32,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
}

impl BuildDate {
    fn from_datetime(timestamp: time::OffsetDateTime) -> Self {
        BuildDate {
            year: timestamp.year() as u32,
            month: u8::from(timestamp.month()) as u32,
            day: timestamp.day() as u32,
            hour: timestamp.hour() as u32,
            minute: timestamp.minute() as u32,
        }
    }

    fn now() -> Self {
        Self::from_datetime(time::OffsetDateTime::now_utc())
    }

    fn build_string(&self) -> String {
        format!(
            "r{}{:02}{:02}.{:02}{:02}",
            self.year, self.month, self.day, self.hour, self.minute
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_find_release_version() {
        let lines = vec![
            "# Start",
            "### Subheading",
            "text",
            "* bullet",
            "## 1.0.0 - release date - yeah really",
            "",
        ];
        let release_version = find_release_version(lines.iter().copied());
        assert_eq!(release_version, Some("1.0.0"));
    }

    #[test]
    fn should_find_release_version_without_dates() {
        let lines = vec!["text", "## 1.0.0", "text", ""];
        let release_version = find_release_version(lines.iter().copied());
        assert_eq!(release_version, Some("1.0.0"));
    }

    #[test]
    fn should_format_version_string_and_image_tag_string() {
        let ver = BuildVersion {
            release: "1.0.5".to_string(),
            build_date: BuildDate {
                year: 2021,
                month: 8,
                day: 7,
                hour: 12,
                minute: 11,
            },
        };
        assert_eq!(&ver.version_string(), "1.0.5+r20210807.1211");
        assert_eq!(&ver.image_tag_string(), "1.0.5-r20210807.1211");
    }

    #[test]
    fn should_get_build_date_from_timestamp() {
        let date = BuildDate::from_datetime(
            time::OffsetDateTime::from_unix_timestamp(1628356266).unwrap(),
        );
        assert_eq!(
            date,
            BuildDate {
                year: 2021,
                month: 8,
                day: 7,
                hour: 17,
                minute: 11
            }
        );
    }
}
