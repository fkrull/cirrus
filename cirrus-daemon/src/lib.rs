use cirrus_core::{model::Config, restic::Restic, secrets::Secrets};
use std::sync::Arc;

pub mod actor;
pub mod job_description;
pub mod jobs;
pub mod queues;

#[derive(Debug, Clone)]
pub struct Daemon {
    pub instance_name: String,
    pub config: Arc<Config>,
    pub restic: Arc<Restic>,
    pub secrets: Arc<Secrets>,
    pub jobs_push: jobs::JobsRunnerPush,
}
