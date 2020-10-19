use cirrus_core::model;

mod backup;
pub use backup::*;

#[derive(Debug, PartialEq, Eq, Clone)]
pub(crate) struct QueueId<'a> {
    pub repo: &'a model::repo::Name,
    pub backup: Option<&'a model::backup::Name>,
}

#[derive(Debug)]
pub enum JobDescription {
    Backup(BackupDescription),
}

impl JobDescription {
    pub(crate) fn queue_id(&self) -> QueueId {
        match self {
            JobDescription::Backup(desc) => desc.queue_id(),
        }
    }

    pub(crate) async fn start_job(self) -> eyre::Result<()> {
        match self {
            JobDescription::Backup(desc) => desc.start_job().await,
        }
    }
}
