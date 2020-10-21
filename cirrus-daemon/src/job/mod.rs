use cirrus_core::model;

mod backup;
pub use backup::*;

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct JobId(uuid::Uuid);

impl JobId {
    pub fn new() -> Self {
        JobId(uuid::Uuid::new_v4())
    }
}

#[derive(Debug, Clone)]
pub struct Job {
    pub id: JobId,
    pub spec: JobSpec,
}

impl Job {
    pub fn new(spec: JobSpec) -> Self {
        Job {
            id: JobId::new(),
            spec,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub(crate) struct QueueId<'a> {
    pub repo: &'a model::repo::Name,
    pub backup: Option<&'a model::backup::Name>,
}

#[derive(Debug, Clone)]
pub enum JobSpec {
    Backup(BackupSpec),
}

impl JobSpec {
    pub(crate) fn queue_id(&self) -> QueueId {
        match self {
            JobSpec::Backup(spec) => spec.queue_id(),
        }
    }

    pub(crate) async fn run_job(self) -> eyre::Result<()> {
        match self {
            JobSpec::Backup(spec) => spec.run_job().await,
        }
    }
}
