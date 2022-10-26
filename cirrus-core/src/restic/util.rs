use super::{Options, Output, Restic};
use tokio::io::{AsyncBufReadExt, BufReader};

impl Restic {
    pub async fn version_string(&self) -> eyre::Result<String> {
        let mut process = self.run(
            None,
            &["version"],
            &Options {
                stdout: Output::Capture,
                ..Default::default()
            },
        )?;
        let mut lines = BufReader::new(
            process
                .stdout()
                .take()
                .expect(" should be present because of params"),
        )
        .lines();
        let mut version = None;
        while let Some(line) = lines.next_line().await? {
            if let Some(v) = version_line(&line) {
                version = Some(v.to_string());
                break;
            }
        }
        process.wait().await?;
        version.ok_or_else(|| eyre::eyre!("couldn't get restic version from output"))
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
