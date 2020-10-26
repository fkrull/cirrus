use crate::job;
use cirrus_actor::{Actor, ActorRef};
use log::info;
use std::collections::HashMap;

#[derive(Debug)]
pub struct RetryHandler {
    job_sink: ActorRef<job::Job>,
    statuschange_sink: ActorRef<job::StatusChange>,
    attempts: HashMap<job::Id, u32>,
}

impl RetryHandler {
    pub fn new(
        job_sink: ActorRef<job::Job>,
        statuschange_sink: ActorRef<job::StatusChange>,
    ) -> Self {
        RetryHandler {
            job_sink,
            statuschange_sink,
            attempts: HashMap::new(),
        }
    }
}

#[async_trait::async_trait]
impl Actor for RetryHandler {
    type Message = job::StatusChange;
    type Error = eyre::Report;

    async fn on_message(&mut self, message: Self::Message) -> Result<(), Self::Error> {
        match message.new_status {
            job::Status::FinishedWithError => {
                let attempt = self.attempts.get(&message.job.id).copied().unwrap_or(1) + 1;
                let max_attempts = message.job.spec.max_attempts();
                if attempt <= max_attempts {
                    let attempts_left = max_attempts - attempt;
                    // We have attempts left so we requeue the job and send a retry message
                    // downstream.
                    info!(
                        "retrying job '{}' ({} more attempts left)",
                        message.job.spec.name(),
                        attempts_left
                    );
                    self.job_sink.send(message.job.clone())?;
                    self.attempts.insert(message.job.id, attempt);
                    self.statuschange_sink.send(job::StatusChange::new(
                        message.job,
                        job::Status::Retried {
                            attempt,
                            attempts_left,
                        },
                    ))?;
                } else {
                    // We have reached the maximum number of attempts for this job so we simply give
                    // up and forward the message.
                    self.attempts.remove(&message.job.id);
                    self.statuschange_sink.send(message)?;
                }
            }
            job::Status::FinishedSuccessfully => {
                self.attempts.remove(&message.job.id);
                self.statuschange_sink.send(message)?;
            }
            _ => self.statuschange_sink.send(message)?,
        }

        Ok(())
    }
}
