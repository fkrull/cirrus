use cirrus_core::model;

mod backup;
pub use backup::*;

#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone)]
pub struct Id(uuid::Uuid);

impl Id {
    pub fn new() -> Self {
        Id(uuid::Uuid::new_v4())
    }
}

#[derive(Debug, Clone)]
pub struct Job {
    pub id: Id,
    pub spec: Spec,
}

impl Job {
    pub fn new(spec: Spec) -> Self {
        Job {
            id: Id::new(),
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
pub enum Spec {
    Backup(BackupSpec),
}

impl Spec {
    pub(crate) fn queue_id(&self) -> QueueId {
        match self {
            Spec::Backup(spec) => spec.queue_id(),
        }
    }

    pub(crate) async fn run_job(self) -> eyre::Result<()> {
        match self {
            Spec::Backup(spec) => spec.run_job().await,
        }
    }

    pub(crate) fn max_attempts(&self) -> u32 {
        3
    }

    pub(crate) fn name(&self) -> &str {
        match self {
            Spec::Backup(spec) => spec.name(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct StatusChange {
    pub job: Job,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub new_status: Status,
}

impl StatusChange {
    pub(crate) fn new(job: Job, new_status: Status) -> Self {
        StatusChange {
            job,
            timestamp: chrono::Utc::now(),
            new_status,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum Status {
    Started,
    FinishedSuccessfully,
    FinishedWithError,
    Retried { attempt: u32, attempts_left: u32 },
}
