use crate::jobs::{repo::JobsRepo, runner::JobsRunnerSender};
use cirrus_core::{model::Config, restic::Restic, secrets::Secrets};
use std::sync::Arc;

pub mod jobs;

#[derive(Debug, Clone)]
pub struct Daemon {
    pub config: Arc<Config>,
    pub restic: Arc<Restic>,
    pub secrets: Arc<Secrets>,
    pub jobs_sender: Arc<JobsRunnerSender>,
    pub jobs_repo: Arc<JobsRepo>,
}
