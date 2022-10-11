use cirrus_core::{config::Config, restic::Restic, secrets::Secrets};
use std::sync::Arc;

pub mod config_reload;
pub mod job;
pub mod scheduler;
pub mod shutdown;
pub mod suspend;

#[derive(Debug, Clone)]
pub struct Daemon {
    pub instance_name: String,
    pub config: Arc<Config>,
    pub restic: Arc<Restic>,
    pub secrets: Arc<Secrets>,
}
