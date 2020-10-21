use super::QueueId;
use cirrus_core::{
    model::{backup, repo},
    restic::Restic,
    secrets::Secrets,
};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct BackupSpec {
    pub restic: Arc<Restic>,
    pub secrets: Arc<Secrets>,
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

    pub(super) async fn run_job(self) -> eyre::Result<()> {
        use cirrus_core::restic::Options;

        let repo_with_secrets = self.secrets.get_secrets(&self.repo)?;
        let process = self.restic.backup(
            repo_with_secrets,
            &self.backup,
            &Options {
                capture_output: false,
                ..Default::default()
            },
        )?;
        process.wait().await?;
        Ok(())
    }
}
