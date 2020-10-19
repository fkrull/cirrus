use crate::job_description::JobDescription;
use cirrus_core::{model::backup, model::repo};
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
struct JobQueue {
    running: Option<RunningJob>,
    queue: VecDeque<JobDescription>,
}

impl JobQueue {
    fn push(&mut self, description: JobDescription) {
        self.queue.push_back(description);
    }

    fn has_running_job(&self) -> bool {
        self.running.is_some()
    }

    fn has_waiting_jobs(&self) -> bool {
        !self.queue.is_empty()
    }

    fn maybe_start_next_job(&mut self) {
        if !self.has_running_job() {
            if let Some(description) = self.queue.pop_front() {
                let job = RunningJob {
                    fut: Box::pin(description.start_job()),
                };
                self.running = Some(job);
            }
        }
    }

    async fn run(&mut self) {
        if let Some(job) = &mut self.running {
            if let Err(error) = (&mut job.fut).await {
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
    repo_queue: JobQueue,
    per_backup_queues: HashMap<backup::Name, JobQueue>,
}

impl PerRepositoryQueue {
    fn push(&mut self, description: JobDescription) {
        match description.queue_id().backup {
            Some(backup) => self
                .per_backup_queues
                .entry(backup.clone())
                .or_default()
                .push(description),
            None => self.repo_queue.push(description),
        }
    }

    fn maybe_start_next_jobs(&mut self) {
        if self.repo_queue.has_waiting_jobs() {
            // if we have repo-wide jobs_prev pending, we try to do them next
            if self.has_running_jobs() {
                // if any jobs_prev are currently running, we do nothing and wait for them to finish
            } else {
                // if no more jobs_prev are running, we enqueue a pending repo-wide job
                self.repo_queue.maybe_start_next_job();
            }
        } else {
            // if we have no repo-wide jobs_prev pending, we run per-backup jobs_prev
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
pub(crate) struct Queues {
    per_repo_queues: HashMap<repo::Name, PerRepositoryQueue>,
}

impl Queues {
    pub(crate) fn push(&mut self, description: JobDescription) {
        self.per_repo_queues
            .entry(description.queue_id().repo.clone())
            .or_default()
            .push(description);
    }

    pub(crate) fn maybe_start_next_jobs(&mut self) {
        // start more jobs_prev as necessary
        self.per_repo_queues
            .values_mut()
            .for_each(|q| q.maybe_start_next_jobs());
    }

    pub(crate) async fn run(&mut self) {
        let jobs = self
            .per_repo_queues
            .values_mut()
            .map(|q| q.run())
            .map(|f| Box::pin(f));
        select_all_or_pending(jobs).await;
    }
}
