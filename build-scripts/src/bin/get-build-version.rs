/// Get the current build version.
#[derive(argh::FromArgs)]
struct Args {}

fn main() -> eyre::Result<()> {
    let changelog = std::fs::read_to_string("CHANGELOG.md")?;
    let release_version = find_release_version(changelog.lines())
        .ok_or_else(|| eyre::eyre!("failed to find release version in changelog file"))?;
    let build_version = BuildVersion {
        release: release_version.to_string(),
        build_date: BuildDate::now(),
    };
    println!("{}", build_version.version_string());
    Ok(())
}

fn find_release_version<'a>(lines: impl Iterator<Item = &'a str>) -> Option<&'a str> {
    lines.filter_map(find_in_line).next()
}

fn find_in_line(line: &str) -> Option<&str> {
    line.trim()
        .strip_prefix("## ")
        .and_then(|s| s.split_once("-"))
        .map(|p| p.0.trim())
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
    fn from_time(time: libc::time_t) -> Self {
        let mut gmtime = libc::tm {
            tm_sec: 0,
            tm_min: 0,
            tm_hour: 0,
            tm_mday: 0,
            tm_mon: 0,
            tm_year: 0,
            tm_wday: 0,
            tm_yday: 0,
            tm_isdst: 0,
        };
        unsafe { libc::gmtime_s(&mut gmtime, &time) };
        BuildDate {
            year: gmtime.tm_year as u32 + 1900,
            month: gmtime.tm_mon as u32 + 1,
            day: gmtime.tm_mday as u32,
            hour: gmtime.tm_hour as u32,
            minute: gmtime.tm_min as u32,
        }
    }

    fn now() -> Self {
        let mut now = 0;
        unsafe { libc::time(&mut now) };
        Self::from_time(now)
    }

    fn build_string(&self) -> String {
        format!(
            "r{}{:02}{:02}.{:02}{:02}",
            self.year, self.month, self.day, self.hour, self.minute
        )
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
struct BuildVersion {
    release: String,
    build_date: BuildDate,
}

impl BuildVersion {
    fn version_string(&self) -> String {
        format!("{}+{}", self.release, self.build_date.build_string())
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
    fn should_format_version_string() {
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
    }

    #[test]
    fn should_get_build_date_from_timestamp() {
        let date = BuildDate::from_time(1628356266);
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
