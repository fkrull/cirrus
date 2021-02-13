use super::QueueId;
use cirrus_core::restic::Event;
use cirrus_core::{
    model::{backup, repo},
    restic::Restic,
    secrets::Secrets,
};
use log::{info, warn};

#[derive(Debug, Clone)]
pub struct BackupSpec {
    pub repo_name: repo::Name,
    pub backup_name: backup::Name,
    pub repo: repo::Definition,
    pub backup: backup::Definition,
}

impl BackupSpec {
    pub(super) fn queue_id(&self) -> QueueId {
        QueueId {
            repo: &self.repo_name,
            backup: Some(&self.backup_name),
        }
    }

    pub(super) async fn run_job(self, restic: &Restic, secrets: &Secrets) -> eyre::Result<()> {
        use cirrus_core::restic::Options;

        let repo_with_secrets = secrets.get_secrets(&self.repo)?;
        let mut process = restic.backup(
            repo_with_secrets,
            &self.backup_name,
            &self.backup,
            &Options {
                capture_output: true,
            },
        )?;

        loop {
            match process.next_event().await? {
                Event::ProcessExit(status) => {
                    return if status.success() {
                        Ok(())
                    } else if let Some(code) = status.code() {
                        Err(eyre::eyre!("restic exited with status {}", code))
                    } else {
                        Err(eyre::eyre!("restic exited with unknown status"))
                    }
                }
                Event::StdoutLine(line) => {
                    info!("{}", line);
                }
                Event::StderrLine(line) => {
                    warn!("{}", line);
                }
            }
        }

        //process.wait().await?;
        //Ok(())
    }

    pub(super) fn name(&self) -> &str {
        &self.backup_name.0
    }
}
