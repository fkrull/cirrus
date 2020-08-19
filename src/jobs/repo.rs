use crate::jobs::Job;
use std::collections::HashMap;
use tokio::sync::RwLock;

#[derive(Debug)]
pub struct JobsRepo {
    jobs: RwLock<HashMap<u64, Job>>,
}

impl JobsRepo {
    pub fn new() -> Self {
        JobsRepo {
            jobs: RwLock::new(HashMap::new()),
        }
    }

    pub async fn save(&self, job: Job) {
        let mut jobs = self.jobs.write().await;
        jobs.insert(job.id, job);
    }
}
