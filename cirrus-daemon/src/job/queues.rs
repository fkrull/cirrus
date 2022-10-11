use crate::job;
use crate::shutdown::{ShutdownAcknowledged, ShutdownRequested};
use cirrus_core::{config, restic::Restic, secrets::Secrets};
use futures::StreamExt as _;
use shindig::Events;
use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
};

#[derive(Debug)]
struct RunningJob {
    job: job::Job,
    cancellation: Option<job::cancellation::Send>,
}

#[derive(Debug)]
struct RunQueue {
    events: Events,
    restic: Arc<Restic>,
    secrets: Arc<Secrets>,
    running: Option<RunningJob>,
    queue: VecDeque<job::Job>,
}

impl RunQueue {
    fn new(events: Events, restic: Arc<Restic>, secrets: Arc<Secrets>) -> Self {
        RunQueue {
            events,
            restic,
            secrets,
            running: None,
            queue: VecDeque::new(),
        }
    }

    // TODO: check if running or enqueued based on label
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
                // TODO: cancellation channel
                let mut runner = job::runner::Runner::new(
                    self.events.clone(),
                    self.restic.clone(),
                    self.secrets.clone(),
                );
                let cloned_job = job.clone();
                let (cancellation_send, cancellation_recv) = job::cancellation::new();
                tokio::spawn(async move { runner.run(cloned_job, cancellation_recv).await });
                self.running = Some(RunningJob {
                    job,
                    cancellation: Some(cancellation_send),
                });
            }
        }
        Ok(())
    }

    fn job_finished(&mut self, job: &job::Job) {
        if let Some(running) = &self.running {
            if &running.job == job {
                self.running = None;
            }
        }
    }

    async fn cancel(&mut self, reason: job::cancellation::Reason) {
        if let Some(cancellation) = self.running.as_mut().and_then(|j| j.cancellation.take()) {
            cancellation.cancel(reason).await;
        }
    }
}

#[derive(Debug)]
struct PerRepositoryQueue {
    events: Events,
    restic: Arc<Restic>,
    secrets: Arc<Secrets>,
    repo_queue: RunQueue,
    per_backup_queues: HashMap<config::backup::Name, RunQueue>,
}

impl PerRepositoryQueue {
    fn new(events: Events, restic: Arc<Restic>, secrets: Arc<Secrets>) -> Self {
        let repo_queue = RunQueue::new(events.clone(), restic.clone(), secrets.clone());
        PerRepositoryQueue {
            events,
            restic,
            secrets,
            repo_queue,
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

    fn job_finished(&mut self, job: &job::Job) {
        self.repo_queue.job_finished(job);
        for queue in self.per_backup_queues.values_mut() {
            queue.job_finished(job);
        }
    }

    async fn cancel(&mut self, reason: job::cancellation::Reason) {
        let mut futs = futures::stream::FuturesUnordered::new();
        futs.push(self.repo_queue.cancel(reason));
        futs.extend(
            self.per_backup_queues
                .values_mut()
                .map(|q| q.cancel(reason)),
        );
        futs.count().await;
    }

    fn has_running_jobs(&self) -> bool {
        self.repo_queue.has_running_job()
            || self.per_backup_queues.values().any(|q| q.has_running_job())
    }
}

#[derive(Debug)]
pub struct JobQueues {
    events: Events,
    restic: Arc<Restic>,
    secrets: Arc<Secrets>,
    per_repo_queues: HashMap<config::repo::Name, PerRepositoryQueue>,
}

impl JobQueues {
    pub fn new(events: Events, restic: Arc<Restic>, secrets: Arc<Secrets>) -> Self {
        JobQueues {
            events,
            restic,
            secrets,
            per_repo_queues: HashMap::new(),
        }
    }

    fn push(&mut self, job: job::Job) {
        self.per_repo_queues
            .entry(job.spec.queue_id().repo.clone())
            .or_insert_with(|| {
                PerRepositoryQueue::new(
                    self.events.clone(),
                    self.restic.clone(),
                    self.secrets.clone(),
                )
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

    fn handle_status_change(&mut self, status_change: job::StatusChange) {
        match status_change.new_status {
            job::Status::Started => {}
            job::Status::FinishedSuccessfully
            | job::Status::FinishedWithError
            | job::Status::Cancelled => self.job_finished(&status_change.job),
        }
    }

    fn job_finished(&mut self, job: &job::Job) {
        for queue in self.per_repo_queues.values_mut() {
            queue.job_finished(job);
        }
    }

    async fn cancel(&mut self, reason: job::cancellation::Reason) {
        let mut futs = futures::stream::FuturesUnordered::new();
        futs.extend(self.per_repo_queues.values_mut().map(|q| q.cancel(reason)));
        futs.count().await;
    }

    #[tracing::instrument(name = "job_queues", skip_all)]
    pub async fn run(&mut self) -> eyre::Result<()> {
        let mut job_recv = self.events.subscribe::<job::Job>();
        let mut status_change_recv = self.events.subscribe::<job::StatusChange>();
        let mut shutdown_recv = self.events.subscribe::<ShutdownRequested>();
        loop {
            tokio::select! {
                job = job_recv.recv() => self.push(job?),
                status_change = status_change_recv.recv() => self.handle_status_change(status_change?),
                shutdown = shutdown_recv.recv() => {
                    let _ = shutdown?;
                    tracing::debug!("received shutdown event");
                    self.cancel(job::cancellation::Reason::Shutdown).await;
                    self.events.send(ShutdownAcknowledged);
                    break Ok(());
                },
            }
            self.maybe_start_next_jobs()?;
        }
    }
}