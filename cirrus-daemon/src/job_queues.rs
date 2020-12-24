use crate::job;
use cirrus_actor::Messages;
use cirrus_core::model;
use log::{error, info};
use std::{
    collections::{HashMap, VecDeque},
    future::Future,
    pin::Pin,
};

async fn select_all_or_pending<F: Future + Unpin>(
    it: impl ExactSizeIterator<Item = F>,
) -> F::Output {
    use futures::{future::pending, future::select_all};
    if it.len() != 0 {
        let (val, _, _) = select_all(it).await;
        val
    } else {
        pending::<F::Output>().await
    }
}

struct RunningJob {
    job: job::Job,
    fut: Pin<Box<dyn Future<Output = eyre::Result<()>> + Send>>,
}

impl std::fmt::Debug for RunningJob {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RunningJob")
            .field("fut", &"<dyn Future>")
            .finish()
    }
}

#[derive(Debug)]
struct RunQueue {
    jobstatus_messages: Messages<job::StatusChange>,
    running: Option<RunningJob>,
    queue: VecDeque<job::Job>,
}

impl RunQueue {
    fn new(jobstatus_messages: Messages<job::StatusChange>) -> Self {
        RunQueue {
            jobstatus_messages,
            running: None,
            queue: VecDeque::new(),
        }
    }

    fn push(&mut self, job: job::Job) {
        self.queue.push_back(job);
    }

    fn has_running_job(&self) -> bool {
        self.running.is_some()
    }

    fn has_waiting_jobs(&self) -> bool {
        !self.queue.is_empty()
    }

    fn maybe_start_next_job(&mut self) -> eyre::Result<()> {
        if !self.has_running_job() {
            if let Some(job) = self.queue.pop_front() {
                let fut = Box::pin(job.spec.clone().run_job());
                self.running = Some(RunningJob {
                    job: job.clone(),
                    fut,
                });
                self.jobstatus_messages
                    .send(job::StatusChange::new(job, job::Status::Started))?;
            }
        }
        Ok(())
    }

    async fn run(&mut self) -> eyre::Result<()> {
        if let Some(running) = &mut self.running {
            let result = (&mut running.fut).await;
            let job = self.running.take().unwrap().job;
            let new_status = match result {
                Ok(_) => {
                    info!("job '{}' finished successfully", job.spec.name());
                    job::Status::FinishedSuccessfully
                }
                Err(error) => {
                    error!("job '{}' failed: {}", job.spec.name(), error);
                    job::Status::FinishedWithError
                }
            };
            self.jobstatus_messages
                .send(job::StatusChange::new(job, new_status))?;
        } else {
            futures::future::pending::<()>().await;
        }
        Ok(())
    }
}

#[derive(Debug)]
struct PerRepositoryQueue {
    jobstatus_messages: Messages<job::StatusChange>,
    repo_queue: RunQueue,
    per_backup_queues: HashMap<model::backup::Name, RunQueue>,
}

impl PerRepositoryQueue {
    fn new(jobstatus_messages: Messages<job::StatusChange>) -> Self {
        PerRepositoryQueue {
            repo_queue: RunQueue::new(jobstatus_messages.clone()),
            jobstatus_messages,
            per_backup_queues: HashMap::new(),
        }
    }

    fn push(&mut self, job: job::Job) {
        let messages = &self.jobstatus_messages;
        match job.spec.queue_id().backup {
            Some(backup) => self
                .per_backup_queues
                .entry(backup.clone())
                .or_insert_with(|| RunQueue::new(messages.clone()))
                .push(job),
            None => self.repo_queue.push(job),
        }
    }

    fn maybe_start_next_jobs(&mut self) -> eyre::Result<()> {
        if self.repo_queue.has_waiting_jobs() {
            // if we have repo-wide jobs pending, we try to do them next
            if self.has_running_jobs() {
                // if any jobs are currently running, we do nothing and wait for them to finish
            } else {
                // if no more jobs are running, we enqueue a pending repo-wide job
                self.repo_queue.maybe_start_next_job()?;
            }
        } else {
            // if we have no repo-wide jobs pending, we run per-backup jobs
            for queue in self.per_backup_queues.values_mut() {
                queue.maybe_start_next_job()?;
            }
        }
        Ok(())
    }

    fn has_running_jobs(&self) -> bool {
        self.repo_queue.has_running_job()
            || self.per_backup_queues.values().any(|q| q.has_running_job())
    }

    async fn run(&mut self) -> eyre::Result<()> {
        use futures::future::select;
        use futures::pin_mut;

        let repo_job = self.repo_queue.run();
        pin_mut!(repo_job);
        let backup_jobs = self
            .per_backup_queues
            .values_mut()
            .map(|q| q.run())
            .map(Box::pin);
        let backup_jobs = select_all_or_pending(backup_jobs);
        pin_mut!(backup_jobs);
        select(repo_job, backup_jobs).await;
        Ok(())
    }
}

#[derive(Debug)]
pub struct JobQueues {
    jobstatus_messages: Messages<job::StatusChange>,
    per_repo_queues: HashMap<model::repo::Name, PerRepositoryQueue>,
}

impl JobQueues {
    pub fn new(jobstatus_messages: Messages<job::StatusChange>) -> Self {
        JobQueues {
            jobstatus_messages,
            per_repo_queues: HashMap::new(),
        }
    }

    fn push(&mut self, job: job::Job) {
        let messages = &self.jobstatus_messages;
        self.per_repo_queues
            .entry(job.spec.queue_id().repo.clone())
            .or_insert_with(|| PerRepositoryQueue::new(messages.clone()))
            .push(job);
    }

    fn maybe_start_next_jobs(&mut self) -> eyre::Result<()> {
        // start more jobs as necessary
        for queue in self.per_repo_queues.values_mut() {
            queue.maybe_start_next_jobs()?;
        }
        Ok(())
    }

    async fn run(&mut self) -> eyre::Result<()> {
        let jobs = self
            .per_repo_queues
            .values_mut()
            .map(|q| q.run())
            .map(Box::pin);
        select_all_or_pending(jobs).await?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl cirrus_actor::Actor for JobQueues {
    type Message = job::Job;
    type Error = eyre::Report;

    async fn on_message(&mut self, job: Self::Message) -> Result<(), Self::Error> {
        self.push(job);
        self.maybe_start_next_jobs()?;
        Ok(())
    }

    async fn on_idle(&mut self) -> Result<(), Self::Error> {
        self.maybe_start_next_jobs()?;
        self.run().await?;
        Ok(())
    }
}
