use crate::jobs::{Job, JobDescription};
use cirrus_core::model::backup;
use std::{
    collections::HashMap,
    sync::atomic::{AtomicU64, Ordering},
};
use tokio::sync::RwLock;

#[derive(Debug)]
pub struct JobsRepo {
    jobs: RwLock<HashMap<u64, Job>>,
    next_id: AtomicU64,
}

impl JobsRepo {
    pub fn new() -> Self {
        JobsRepo {
            jobs: RwLock::new(HashMap::new()),
            next_id: AtomicU64::new(1),
        }
    }

    pub async fn save(&self, job: Job) {
        let mut jobs = self.jobs.write().await;
        jobs.insert(job.id, job);
    }

    pub async fn backup_jobs(&self, backup_name: &backup::Name) -> Vec<Job> {
        let jobs = self.jobs.read().await;
        let mut backup_jobs = jobs
            .values()
            .filter(|&job| match &job.description {
                JobDescription::Backup { name, .. } => name == backup_name,
            })
            .cloned()
            .collect::<Vec<_>>();
        backup_jobs.sort_by_key(|job| job.started);
        backup_jobs.reverse();
        backup_jobs
    }

    pub fn next_id(&self) -> u64 {
        self.next_id.fetch_add(1, Ordering::Relaxed)
    }
}
