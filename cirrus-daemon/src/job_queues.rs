use crate::job;
use cirrus_core::{config, restic::Restic, secrets::Secrets};
use shindig::Events;
use std::{
    collections::{HashMap, VecDeque},
    future::Future,
    pin::Pin,
    sync::Arc,
};
use tracing::{error, info, info_span};
use tracing_futures::Instrument;

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
    result: Option<eyre::Result<()>>,
}

impl std::fmt::Debug for RunningJob {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RunningJob")
            .field("job", &self.job)
            .field("fut", &"<dyn Future>")
            .field("result", &self.result)
            .finish()
    }
}

impl RunningJob {
    async fn run(&mut self) {
        if self.result.is_none() {
            self.result = Some((&mut self.fut).await);
        }
    }
}

#[derive(Debug)]
struct RunQueue {
    restic: Arc<Restic>,
    secrets: Arc<Secrets>,
    events: Events,
    running: Option<RunningJob>,
    queue: VecDeque<job::Job>,
}

impl RunQueue {
    fn new(events: Events, restic: Arc<Restic>, secrets: Arc<Secrets>) -> Self {
        RunQueue {
            restic,
            secrets,
            events,
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
                info!("running '{}'", job.spec.label());
                let fut = Box::pin(
                    job.spec
                        .clone()
                        .run_job(self.restic.clone(), self.secrets.clone())
                        .instrument(info_span!(
                            "job",
                            id = %job.id,
                            label = %job.spec.label()
                        )),
                );
                self.running = Some(RunningJob {
                    job: job.clone(),
                    fut,
                    result: None,
                });
                self.events
                    .send(job::StatusChange::new(job, job::Status::Started));
            }
        }
        Ok(())
    }

    fn clean_finished_job(&mut self) -> eyre::Result<()> {
        if let Some(running_job) = &mut self.running {
            let result = running_job.result.take();
            if let Some(result) = result {
                let job = self.running.take().unwrap().job;
                let new_status = match result {
                    Ok(_) => {
                        info!("'{}' finished successfully", job.spec.label());
                        job::Status::FinishedSuccessfully
                    }
                    Err(error) => {
                        error!("'{}' failed: {}", job.spec.label(), error);
                        job::Status::FinishedWithError
                    }
                };
                self.events.send(job::StatusChange::new(job, new_status));
            }
        }
        Ok(())
    }

    async fn poll_job(&mut self) {
        if let Some(running) = &mut self.running {
            running.run().await;
        } else {
            futures::future::pending::<()>().await;
        }
    }
}

#[derive(Debug)]
struct PerRepositoryQueue {
    restic: Arc<Restic>,
    secrets: Arc<Secrets>,
    events: Events,
    repo_queue: RunQueue,
    per_backup_queues: HashMap<config::backup::Name, RunQueue>,
}

impl PerRepositoryQueue {
    fn new(events: Events, restic: Arc<Restic>, secrets: Arc<Secrets>) -> Self {
        let repo_queue = RunQueue::new(events.clone(), restic.clone(), secrets.clone());
        PerRepositoryQueue {
            restic,
            secrets,
            repo_queue,
            events,
            per_backup_queues: HashMap::new(),
        }
    }

    fn push(&mut self, job: job::Job) {
        let restic = &self.restic;
        let secrets = &self.secrets;
        match job.spec.queue_id().backup {
            Some(backup) => self
                .per_backup_queues
                .entry(backup.clone())
                .or_insert_with(|| {
                    RunQueue::new(self.events.clone(), restic.clone(), secrets.clone())
                })
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

    fn clean_finished_jobs(&mut self) -> eyre::Result<()> {
        self.repo_queue.clean_finished_job()?;
        for queue in self.per_backup_queues.values_mut() {
            queue.clean_finished_job()?;
        }
        Ok(())
    }

    fn has_running_jobs(&self) -> bool {
        self.repo_queue.has_running_job()
            || self.per_backup_queues.values().any(|q| q.has_running_job())
    }

    async fn poll_jobs(&mut self) {
        use futures::future::select;
        use futures::pin_mut;

        let repo_job = self.repo_queue.poll_job();
        pin_mut!(repo_job);
        let backup_jobs = self
            .per_backup_queues
            .values_mut()
            .map(|q| q.poll_job())
            .map(Box::pin);
        let backup_jobs = select_all_or_pending(backup_jobs);
        pin_mut!(backup_jobs);
        select(repo_job, backup_jobs).await;
    }
}

#[derive(Debug)]
pub struct JobQueues {
    restic: Arc<Restic>,
    secrets: Arc<Secrets>,
    events: Events,
    per_repo_queues: HashMap<config::repo::Name, PerRepositoryQueue>,
}

impl JobQueues {
    pub fn new(events: Events, restic: Arc<Restic>, secrets: Arc<Secrets>) -> Self {
        JobQueues {
            restic,
            secrets,
            events,
            per_repo_queues: HashMap::new(),
        }
    }

    fn push(&mut self, job: job::Job) {
        let restic = &self.restic;
        let secrets = &self.secrets;
        self.per_repo_queues
            .entry(job.spec.queue_id().repo.clone())
            .or_insert_with(|| {
                PerRepositoryQueue::new(self.events.clone(), restic.clone(), secrets.clone())
            })
            .push(job);
    }

    fn maybe_start_next_jobs(&mut self) -> eyre::Result<()> {
        // start more jobs as necessary
        for queue in self.per_repo_queues.values_mut() {
            queue.maybe_start_next_jobs()?;
        }
        Ok(())
    }

    fn clean_finished_jobs(&mut self) -> eyre::Result<()> {
        for queue in self.per_repo_queues.values_mut() {
            queue.clean_finished_jobs()?;
        }
        Ok(())
    }

    async fn poll_jobs(&mut self) {
        let jobs = self
            .per_repo_queues
            .values_mut()
            .map(|q| q.poll_jobs())
            .map(Box::pin);
        select_all_or_pending(jobs).await;
    }

    pub async fn run(&mut self) -> eyre::Result<()> {
        let mut job_recv = self.events.subscribe::<job::Job>();
        loop {
            tokio::select! {
                job = job_recv.recv() => self.push(job?),
                _ = self.poll_jobs() => self.clean_finished_jobs()?,
            }
            self.maybe_start_next_jobs()?;
        }
    }
}
