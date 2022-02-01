use cirrus_core::{config, restic::Restic, secrets::Secrets};
use std::sync::Arc;
use time::OffsetDateTime;

mod backup;
pub use backup::*;

#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone)]
pub struct Id(uuid::Uuid);

impl Default for Id {
    fn default() -> Self {
        Id(uuid::Uuid::new_v4())
    }
}

impl std::fmt::Display for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl Id {
    pub fn new() -> Self {
        Default::default()
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

impl PartialEq for Job {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub(crate) struct QueueId<'a> {
    pub repo: &'a config::repo::Name,
    pub backup: Option<&'a config::backup::Name>,
}

#[derive(Debug, Clone)]
pub enum Spec {
    Backup(BackupSpec),
}

impl From<BackupSpec> for Spec {
    fn from(spec: BackupSpec) -> Self {
        Spec::Backup(spec)
    }
}

impl Spec {
    pub(crate) fn queue_id(&self) -> QueueId {
        match self {
            Spec::Backup(spec) => spec.queue_id(),
        }
    }

    pub(crate) async fn run_job(
        self,
        restic: Arc<Restic>,
        secrets: Arc<Secrets>,
    ) -> eyre::Result<()> {
        match self {
            Spec::Backup(spec) => spec.run_job(&restic, &secrets).await,
        }
    }

    pub fn label(&self) -> String {
        match self {
            Spec::Backup(spec) => format!("backup.{}", spec.name()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct StatusChange {
    pub job: Job,
    pub timestamp: OffsetDateTime,
    pub new_status: Status,
}

impl StatusChange {
    pub(crate) fn new(job: Job, new_status: Status) -> Self {
        StatusChange {
            job,
            timestamp: OffsetDateTime::now_utc(),
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
