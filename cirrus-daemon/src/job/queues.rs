use crate::{
    job,
    shutdown::{ShutdownAcknowledged, ShutdownRequested},
    suspend::Suspend,
};
use cirrus_core::{config, restic::Restic, secrets::Secrets};
use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
};
use tokio::sync::oneshot;

const DEFAULT_PARALLEL_JOBS: u32 = 3;

#[derive(Debug)]
struct RunningJob {
    job: job::Job,
    cancellation: Option<oneshot::Sender<job::CancellationReason>>,
}

#[derive(Debug)]
struct RepositoryQueue {
    sender: events::Sender,
    restic: Arc<Restic>,
    secrets: Arc<Secrets>,
    queue: VecDeque<job::Job>,
    parallel_jobs: usize,
    running: HashMap<job::Id, RunningJob>,
}

impl RepositoryQueue {
    fn new(
        repo: &config::repo::Definition,
        sender: events::Sender,
        restic: Arc<Restic>,
        secrets: Arc<Secrets>,
    ) -> Self {
        let parallel_jobs = repo.parallel_jobs.unwrap_or(DEFAULT_PARALLEL_JOBS) as usize;
        RepositoryQueue {
            sender,
            restic,
            secrets,
            queue: VecDeque::new(),
            parallel_jobs,
            running: HashMap::with_capacity(parallel_jobs),
        }
    }

    fn push(&mut self, job: job::Job) {
        if self.running.values().any(|r| &r.job.spec == &job.spec) {
            tracing::info!(id = %job.id, label = job.spec.label(), "job spec is currently running, not enqueuing it");
            return;
        }
        if self.queue.iter().any(|j| &j.spec == &job.spec) {
            tracing::info!(id = %job.id, label = job.spec.label(), "job spec is currently in the queue, not enqueuing it again");
            return;
        }
        tracing::info!(id = %job.id, label = job.spec.label(), "enqueuing");
        self.queue.push_back(job);
    }

    fn has_running_jobs(&self) -> bool {
        !self.running.is_empty()
    }

    fn start_more_jobs(&mut self) -> eyre::Result<()> {
        while self.running.len() < self.parallel_jobs {
            tracing::debug!(
                running = self.running.len(),
                parallel_jobs = self.parallel_jobs,
                "running more jobs if available"
            );
            if let Some(job) = self.queue.pop_front() {
                tracing::info!(id = %job.id, label = job.spec.label(), "starting job");
                let mut runner = job::runner::Runner::new(
                    self.sender.clone(),
                    self.restic.clone(),
                    self.secrets.clone(),
                );
                let cloned_job = job.clone();
                let (send, recv) = oneshot::channel();
                tokio::spawn(async move { runner.run(cloned_job, recv).await });
                self.running.insert(
                    job.id.clone(),
                    RunningJob {
                        job,
                        cancellation: Some(send),
                    },
                );
            } else {
                break;
            }
        }
        Ok(())
    }

    fn job_finished(&mut self, job: &job::Job, readd_to_queue: bool) {
        if let Some((_, running_job)) = self.running.remove_entry(&job.id) {
            if readd_to_queue {
                tracing::debug!(
                    id = %running_job.job.id,
                    label = ?running_job.job.spec.label(),
                    "adding job to front of queue again"
                );
                self.queue.push_front(running_job.job);
            }
        }
    }

    fn cancel_all(&mut self, reason: job::CancellationReason) {
        for running_job in self.running.values_mut() {
            if let Some(cancel) = running_job.cancellation.take() {
                if let Err(_) = cancel.send(reason) {
                    tracing::warn!("cancellation receiver was dropped, job could not be cancelled");
                }
            }
        }
    }
}

events::subscriptions! {
    Job: job::Job,
    StatusChange: job::StatusChange,
    Suspend,
    ShutdownRequested,
}

#[derive(Debug)]
pub struct JobQueues {
    events: Subscriptions,
    restic: Arc<Restic>,
    secrets: Arc<Secrets>,
    suspend: Suspend,
    repo_queues: HashMap<config::repo::Name, RepositoryQueue>,
}

impl JobQueues {
    pub fn new(
        events: &mut events::Builder,
        restic: Arc<Restic>,
        secrets: Arc<Secrets>,
        suspend: Suspend,
    ) -> Self {
        JobQueues {
            events: Subscriptions::subscribe(events),
            restic,
            secrets,
            suspend,
            repo_queues: HashMap::new(),
        }
    }

    fn push(&mut self, job: job::Job) {
        self.repo_queues
            .entry(job.spec.repo_name().clone())
            .or_insert_with(|| {
                RepositoryQueue::new(
                    job.spec.repo(),
                    self.events.sender.clone(),
                    self.restic.clone(),
                    self.secrets.clone(),
                )
            })
            .push(job);
    }

    fn start_more_jobs(&mut self) -> eyre::Result<()> {
        // start no jobs if suspended
        if self.suspend.is_suspended() {
            return Ok(());
        }
        // start more jobs as necessary
        for queue in self.repo_queues.values_mut() {
            queue.start_more_jobs()?;
        }
        Ok(())
    }

    fn job_finished(&mut self, job: &job::Job, readd_to_queue: bool) {
        for queue in self.repo_queues.values_mut() {
            queue.job_finished(job, readd_to_queue);
        }
    }

    fn cancel_all(&mut self, reason: job::CancellationReason) {
        for queue in self.repo_queues.values_mut() {
            queue.cancel_all(reason);
        }
    }

    fn has_running_jobs(&self) -> bool {
        self.repo_queues.values().any(|q| q.has_running_jobs())
    }

    fn handle_status_change(&mut self, status_change: job::StatusChange) {
        match status_change.new_status {
            job::Status::Started => {}
            job::Status::Cancelled(job::CancellationReason::Suspend) => {
                // jobs that were suspended will restart afterwards
                self.job_finished(&status_change.job, true)
            }
            job::Status::FinishedSuccessfully
            | job::Status::FinishedWithError
            | job::Status::Cancelled(_) => self.job_finished(&status_change.job, false),
        }
    }

    fn handle_suspend(&mut self, suspend: Suspend) {
        self.suspend = suspend;
        if suspend.is_suspended() {
            self.cancel_all(job::CancellationReason::Suspend);
        }
    }

    #[tracing::instrument(skip(self))]
    async fn handle_shutdown(&mut self, _shutdown: ShutdownRequested) -> eyre::Result<()> {
        tracing::debug!("received shutdown event");
        self.cancel_all(job::CancellationReason::Shutdown);
        // process status changes until we're out of running jobs
        while self.has_running_jobs() {
            tracing::debug!("still have running jobs...");
            let status_change = self.events.StatusChange.recv().await?;
            self.handle_status_change(status_change);
        }
        self.events.send(ShutdownAcknowledged);
        tracing::debug!("shutdown acknowledged");
        Ok(())
    }

    #[tracing::instrument(name = "JobQueues", skip_all)]
    pub async fn run(&mut self) -> eyre::Result<()> {
        loop {
            tokio::select! {
                job = self.events.Job.recv() => self.push(job?),
                status_change = self.events.StatusChange.recv() => self.handle_status_change(status_change?),
                suspend = self.events.Suspend.recv() => self.handle_suspend(suspend?),
                shutdown = self.events.ShutdownRequested.recv() => {
                    self.handle_shutdown(shutdown?).await?;
                    break Ok(());
                },
            }
            self.start_more_jobs()?;
        }
    }
}
