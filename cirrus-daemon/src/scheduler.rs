use crate::job_description::JobDescription;
use cirrus_actor::ActorRef;
use cirrus_core::{model::Config, restic::Restic, secrets::Secrets};
use std::sync::Arc;

#[derive(Debug)]
pub struct Scheduler {
    config: Arc<Config>,
    restic: Arc<Restic>,
    secrets: Arc<Secrets>,
    jobs: ActorRef<JobDescription>,
}

impl Scheduler {
    pub fn new(
        config: Arc<Config>,
        restic: Arc<Restic>,
        secrets: Arc<Secrets>,
        jobs: ActorRef<JobDescription>,
    ) -> Self {
        Scheduler {
            config,
            restic,
            secrets,
            jobs,
        }
    }

    pub async fn run(&mut self) -> eyre::Result<()> {
        Ok(())
    }
}
