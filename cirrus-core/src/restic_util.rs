use crate::restic::{Event, Options, Restic};

pub async fn restic_version(restic: &Restic) -> eyre::Result<String> {
    let mut process = restic.run(
        None,
        &["version"],
        &Options {
            capture_output: true,
            ..Default::default()
        },
    )?;

    let mut version_string = None;
    loop {
        match process.next_event().await? {
            Event::StdoutLine(line) => {
                if let Some(v) = get_version(&line) {
                    version_string = Some(v.to_owned())
                }
            }
            Event::ProcessExit(_) => break,
            _ => {}
        }
    }

    version_string.ok_or_else(|| eyre::eyre!("couldn't get restic version from output"))
}

fn get_version(line: &str) -> Option<&str> {
    let line = line.trim();
    if line.is_empty() {
        None
    } else if line.starts_with("restic ") {
        Some(line[6..].trim_start())
    } else {
        Some(line)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_get_no_version_from_whitespace_string() {
        assert_eq!(get_version("      \t  "), None);
    }

    #[test]
    fn should_strip_leading_restic_from_version_string() {
        assert_eq!(get_version("   restic 0.12 etc   "), Some("0.12 etc"));
    }

    #[test]
    fn should_not_strip_leading_restic_if_not_followed_by_space() {
        assert_eq!(get_version("resticx 0.12 etc   "), Some("resticx 0.12 etc"));
    }

    #[test]
    fn should_return_version_string_unchanged_if_not_starting_with_leading_restic() {
        assert_eq!(
            get_version("Reeestic 0.13 yay   "),
            Some("Reeestic 0.13 yay")
        );
    }
}
