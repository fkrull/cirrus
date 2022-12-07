use cirrus_core::config::{backup, repo};
use std::time::Duration;
use time::OffsetDateTime;

pub mod queues;
mod runner;

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

#[derive(Debug, Clone, Eq, PartialEq)]
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

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Spec {
    Backup(BackupSpec),
    FilesIndex(FilesIndexSpec),
}

impl From<BackupSpec> for Spec {
    fn from(spec: BackupSpec) -> Self {
        Spec::Backup(spec)
    }
}

impl From<FilesIndexSpec> for Spec {
    fn from(spec: FilesIndexSpec) -> Self {
        Spec::FilesIndex(spec)
    }
}

impl Spec {
    pub(crate) fn repo_name(&self) -> &repo::Name {
        match self {
            Spec::Backup(spec) => &spec.repo_name,
            Spec::FilesIndex(spec) => &spec.repo_name,
        }
    }

    pub(crate) fn repo(&self) -> &repo::Definition {
        match self {
            Spec::Backup(spec) => &spec.repo,
            Spec::FilesIndex(spec) => &spec.repo,
        }
    }

    pub fn label(&self) -> String {
        match self {
            Spec::Backup(spec) => format!("backup.{}", spec.backup_name.0),
            Spec::FilesIndex(spec) => format!("files-index.{}", spec.repo_name.0),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BackupSpec {
    pub repo_name: repo::Name,
    pub backup_name: backup::Name,
    pub repo: repo::Definition,
    pub backup: backup::Definition,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FilesIndexSpec {
    pub repo_name: repo::Name,
    pub repo: repo::Definition,
    pub max_age: Option<Duration>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
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
    Cancelled(CancellationReason),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum CancellationReason {
    Shutdown,
    Suspend,
}
