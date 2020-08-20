use crate::jobs::Job;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
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

    pub fn next_id(&self) -> u64 {
        self.next_id.fetch_add(1, Ordering::Relaxed)
    }
}
