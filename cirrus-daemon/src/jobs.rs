use crate::actor::Actor;
use crate::job_description::JobDescription;
use crate::queues::Queues;
use async_trait::async_trait;
use log::{error, info};
use std::convert::Infallible;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

#[derive(Debug, Default)]
pub struct JobsRunner {
    //recv: UnboundedReceiver<JobDescription>,
    queues: Queues,
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

    async fn on_close(&mut self) -> Result<(), Infallible> {
        info!("stopping job runner because all send ends were closed");
        Ok(())
    }

    async fn on_idle(&mut self) -> Result<(), Infallible> {
        self.queues.maybe_start_next_jobs();
        self.queues.run().await;
        Ok(())
    }
}

/*impl JobsRunner {
    pub fn new() -> (JobsRunner, JobsRunnerPush) {
        use tokio::sync::mpsc::unbounded_channel;
        let (send, recv) = unbounded_channel();
        let runner = JobsRunner {
            recv,
            queues: Queues::default(),
        };
        let push = JobsRunnerPush(send);
        (runner, push)
    }

    pub async fn run(&mut self) {
        use futures::{
            future::{select, Either},
            pin_mut,
        };

        loop {
            self.queues.maybe_start_next_jobs();

            let recv = self.recv.recv();
            pin_mut!(recv);
            let running_jobs = self.queues.run();
            pin_mut!(running_jobs);

            match select(running_jobs, recv).await {
                Either::Right((None, _)) => {
                    info!("stopping job runner because all send ends were closed");
                    break;
                }
                _ => (),
            }
        }
    }
}*/

/*#[derive(Debug, Clone)]
pub struct JobsRunnerPush(UnboundedSender<JobDescription>);

impl JobsRunnerPush {
    pub fn push(&self, description: JobDescription) {
        if let Err(err) = self.0.send(description) {
            error!(
                "enqueuing a job failed (was the job runner shut down?): {}",
                err
            );
        }
    }
}
*/
