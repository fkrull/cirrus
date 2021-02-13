use cirrus_core::{model::Config, restic::Restic, secrets::Secrets};
use std::sync::Arc;

pub mod configreload;
pub mod daemon_config;
pub mod job;
pub mod job_queues;
pub mod scheduler;

#[derive(Debug, Clone)]
pub struct Daemon {
    pub instance_name: String,
    pub config: Arc<Config>,
    pub restic: Arc<Restic>,
    pub secrets: Arc<Secrets>,
}
