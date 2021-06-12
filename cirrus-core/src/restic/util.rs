use super::{Event, Options, Restic};
use futures::{pin_mut, prelude::*};

impl Restic {
    pub async fn version_string(&self) -> eyre::Result<String> {
        let version_lines = self
            .run(
                None,
                &["version"],
                &Options {
                    capture_output: true,
                    ..Default::default()
                },
            )?
            .map_err(eyre::Report::from)
            .try_filter_map(|ev| async move {
                match ev {
                    Event::StdoutLine(line) => Ok(version_line(&line).map(String::from)),
                    Event::StderrLine(_) => Ok(None),
                }
            });
        pin_mut!(version_lines);
        version_lines
            .next()
            .await
            .unwrap_or_else(|| Err(eyre::eyre!("couldn't get restic version from output")))
    }
}

fn version_line(line: &str) -> Option<&str> {
    Some(line.trim()).filter(|s| !s.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_get_no_version_from_whitespace_string() {
        assert_eq!(version_line("      \t  "), None);
    }

    #[test]
    fn should_return_version_string_after_trimming_whitespace() {
        assert_eq!(version_line("restic 0.13   "), Some("restic 0.13"));
    }
}
