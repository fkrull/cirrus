use cirrus_core::{model, Timestamp};

pub mod repo;
pub mod runner;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum JobDescription {
    Backup {
        backup: model::backup::Definition,
        repo: model::repo::Definition,
    },
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum JobStatus {
    FailedToStart,
    InternalError,
    Running,
    Successful,
    Error,
}

impl JobStatus {
    fn is_running(&self) -> bool {
        match self {
            JobStatus::Running => true,
            _ => false,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Job {
    pub id: u64,
    pub description: JobDescription,
    pub status: JobStatus,
    pub started: Timestamp,
    pub finished: Option<Timestamp>,
}

impl Job {
    fn is_finished(&self) -> bool {
        !self.status.is_running()
    }

    fn finish(&mut self, status: JobStatus) {
        self.finished = Some(cirrus_core::timestamp::now());
        self.status = status;
    }
}
