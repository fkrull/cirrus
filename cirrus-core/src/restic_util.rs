use crate::restic::{Event, Options, Restic};
use futures::{pin_mut, prelude::*};

pub async fn restic_version(restic: &Restic) -> eyre::Result<String> {
    let version_lines = restic
        .run(
            None,
            &["version"],
            &Options {
                capture_output: true,
                ..Default::default()
            },
        )?
        .try_filter_map(|ev| async move {
            match ev {
                Event::StdoutLine(line) => Ok(get_version(&line).map(String::from)),
                Event::StderrLine(_) => Ok(None),
            }
        })
        .map_err(eyre::Report::from);
    pin_mut!(version_lines);
    version_lines
        .next()
        .await
        .unwrap_or_else(|| Err(eyre::eyre!("couldn't get restic version from output")))
}

fn get_version(line: &str) -> Option<&str> {
    Some(line.trim()).filter(|s| !s.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_get_no_version_from_whitespace_string() {
        assert_eq!(get_version("      \t  "), None);
    }

    #[test]
    fn should_return_version_string_after_trimming_whitespace() {
        assert_eq!(get_version("restic 0.13   "), Some("restic 0.13"));
    }
}
