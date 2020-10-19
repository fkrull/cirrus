use super::QueueId;
use cirrus_core::model::{backup, repo};

#[derive(Debug)]
pub struct BackupDescription {
    pub repo_name: repo::Name,
    pub backup_name: backup::Name,
    pub repo: repo::Definition,
    pub backup: backup::Definition,
}

impl BackupDescription {
    pub(super) fn queue_id(&self) -> QueueId {
        QueueId {
            repo: &self.repo_name,
            backup: Some(&self.backup_name),
        }
    }

    pub(super) async fn start_job(self) {
        todo!()
    }
}
