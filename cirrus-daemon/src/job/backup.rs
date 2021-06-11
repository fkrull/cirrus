use super::QueueId;
use cirrus_core::{
    model::{backup, repo},
    restic::{Event, Restic, Verbosity},
    secrets::Secrets,
};
use futures::prelude::*;
use tracing::{info, warn};

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
                verbose: Verbosity::V,
                ..Default::default()
            },
        )?;

        while let Some(event) = process.next().await {
            match event? {
                Event::StdoutLine(line) => {
                    info!("{}", line);
                }
                Event::StderrLine(line) => {
                    warn!("{}", line);
                }
            }
        }

        process.check_wait().await
    }

    pub fn name(&self) -> &str {
        &self.backup_name.0
    }
}
