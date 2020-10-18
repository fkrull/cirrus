use cirrus_core::model::{backup, repo};
use std::{future::Future, pin::Pin};

#[derive(Debug, PartialEq, Eq, Clone)]
pub(crate) struct QueueId<'a> {
    pub repo: &'a repo::Name,
    pub backup: Option<&'a backup::Name>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum JobDescription {
    Backup {
        repo_name: repo::Name,
        backup_name: backup::Name,
        repo: repo::Definition,
        backup: backup::Definition,
    },
}

impl JobDescription {
    pub(crate) fn queue_id(&self) -> QueueId {
        match self {
            JobDescription::Backup {
                repo_name,
                backup_name,
                ..
            } => QueueId {
                repo: repo_name,
                backup: Some(backup_name),
            },
        }
    }

    pub(crate) fn start_job(self) -> Pin<Box<dyn Future<Output = ()> + Send>> {
        todo!()
    }
}
