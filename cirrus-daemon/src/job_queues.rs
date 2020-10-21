use crate::job::Job;
use cirrus_core::model;
use log::error;
use std::{
    collections::{HashMap, VecDeque},
    future::Future,
    pin::Pin,
};

async fn select_all_or_pending<F: Future + Unpin>(it: impl ExactSizeIterator<Item = F>) {
    use futures::{future::pending, future::select_all};
    if it.len() != 0 {
        select_all(it).await;
    } else {
        pending::<()>().await;
    }
}

struct RunningJob {
    _job: Job,
    fut: Pin<Box<dyn Future<Output = eyre::Result<()>> + Send>>,
}

impl std::fmt::Debug for RunningJob {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RunningJob")
            .field("fut", &"<dyn Future>")
            .finish()
    }
}

#[derive(Debug, Default)]
struct RunQueue {
    running: Option<RunningJob>,
    queue: VecDeque<Job>,
}

impl RunQueue {
    fn push(&mut self, job: Job) {
        self.queue.push_back(job);
    }

    fn has_running_job(&self) -> bool {
        self.running.is_some()
    }

    fn has_waiting_jobs(&self) -> bool {
        !self.queue.is_empty()
    }

    fn maybe_start_next_job(&mut self) {
        if !self.has_running_job() {
            if let Some(job) = self.queue.pop_front() {
                let fut = Box::pin(job.spec.clone().run_job());
                self.running = Some(RunningJob { _job: job, fut });
            }
        }
    }

    async fn run(&mut self) {
        if let Some(running) = &mut self.running {
            if let Err(error) = (&mut running.fut).await {
                error!("job failed: {}", error);
            }
            self.running = None;
        } else {
            futures::future::pending().await
        }
    }
}

#[derive(Debug, Default)]
struct PerRepositoryQueue {
    repo_queue: RunQueue,
    per_backup_queues: HashMap<model::backup::Name, RunQueue>,
}

impl PerRepositoryQueue {
    fn push(&mut self, job: Job) {
        match job.spec.queue_id().backup {
            Some(backup) => self
                .per_backup_queues
                .entry(backup.clone())
                .or_default()
                .push(job),
            None => self.repo_queue.push(job),
        }
    }

    fn maybe_start_next_jobs(&mut self) {
        if self.repo_queue.has_waiting_jobs() {
            // if we have repo-wide jobs pending, we try to do them next
            if self.has_running_jobs() {
                // if any jobs are currently running, we do nothing and wait for them to finish
            } else {
                // if no more jobs are running, we enqueue a pending repo-wide job
                self.repo_queue.maybe_start_next_job();
            }
        } else {
            // if we have no repo-wide jobs pending, we run per-backup jobs
            self.per_backup_queues
                .values_mut()
                .for_each(|q| q.maybe_start_next_job());
        }
    }

    fn has_running_jobs(&self) -> bool {
        self.repo_queue.has_running_job()
            || self.per_backup_queues.values().any(|q| q.has_running_job())
    }

    async fn run(&mut self) {
        use futures::future::select;
        use futures::pin_mut;

        let repo_job = self.repo_queue.run();
        pin_mut!(repo_job);
        let backup_jobs = self
            .per_backup_queues
            .values_mut()
            .map(|q| q.run())
            .map(|f| Box::pin(f));
        let backup_jobs = select_all_or_pending(backup_jobs);
        pin_mut!(backup_jobs);
        select(repo_job, backup_jobs).await;
    }
}

#[derive(Debug, Default)]
pub struct JobQueues {
    per_repo_queues: HashMap<model::repo::Name, PerRepositoryQueue>,
}

impl JobQueues {
    pub fn new() -> Self {
        Self::default()
    }

    fn push(&mut self, job: Job) {
        self.per_repo_queues
            .entry(job.spec.queue_id().repo.clone())
            .or_default()
            .push(job);
    }

    fn maybe_start_next_jobs(&mut self) {
        // start more jobs as necessary
        self.per_repo_queues
            .values_mut()
            .for_each(|q| q.maybe_start_next_jobs());
    }

    async fn run(&mut self) {
        let jobs = self
            .per_repo_queues
            .values_mut()
            .map(|q| q.run())
            .map(|f| Box::pin(f));
        select_all_or_pending(jobs).await;
    }
}

#[async_trait::async_trait]
impl cirrus_actor::Actor for JobQueues {
    type Message = Job;
    type Error = std::convert::Infallible;

    async fn on_message(&mut self, job: Self::Message) -> Result<(), Self::Error> {
        self.push(job);
        self.maybe_start_next_jobs();
        Ok(())
    }

    async fn on_idle(&mut self) -> Result<(), Self::Error> {
        self.maybe_start_next_jobs();
        self.run().await;
        Ok(())
    }
}
