use crate::{job_description::JobDescription, queues::Queues};
use async_trait::async_trait;
use cirrus_actor::{Actor, ActorInstance, ActorRef};
use std::convert::Infallible;

pub type JobsRunnerInstance = ActorInstance<JobsRunner>;
pub type JobsRunnerRef = ActorRef<JobDescription>;

#[derive(Debug, Default)]
pub struct JobsRunner {
    queues: Queues,
}

impl JobsRunner {
    pub fn new() -> (JobsRunnerInstance, JobsRunnerRef) {
        ActorInstance::new(JobsRunner::default())
    }
}

#[async_trait]
impl Actor for JobsRunner {
    type Message = JobDescription;
    type Error = Infallible;

    async fn on_message(&mut self, description: JobDescription) -> Result<(), Infallible> {
        self.queues.push(description);
        self.queues.maybe_start_next_jobs();
        Ok(())
    }

    async fn on_idle(&mut self) -> Result<(), Infallible> {
        self.queues.maybe_start_next_jobs();
        self.queues.run().await;
        Ok(())
    }
}
