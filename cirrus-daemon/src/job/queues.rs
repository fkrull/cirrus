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

#[derive(Debug)]
struct RunningJob {
    job: job::Job,
    cancellation: Option<oneshot::Sender<job::CancellationReason>>,
}

#[derive(Debug)]
struct RunQueue {
    sender: events::Sender,
    restic: Arc<Restic>,
    secrets: Arc<Secrets>,
    running: Option<RunningJob>,
    queue: VecDeque<job::Job>,
}

impl RunQueue {
    fn new(sender: events::Sender, restic: Arc<Restic>, secrets: Arc<Secrets>) -> Self {
        RunQueue {
            sender,
            restic,
            secrets,
            running: None,
            queue: VecDeque::new(),
        }
    }

    // TODO: don't enqueue job that's already running or enqueued
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
                tracing::info!(id = %job.id, label = job.spec.label(), "starting job");
                let mut runner = job::runner::Runner::new(
                    self.sender.clone(),
                    self.restic.clone(),
                    self.secrets.clone(),
                );
                let cloned_job = job.clone();
                let (send, recv) = oneshot::channel();
                tokio::spawn(async move { runner.run(cloned_job, recv).await });
                self.running = Some(RunningJob {
                    job,
                    cancellation: Some(send),
                });
            }
        }
        Ok(())
    }

    fn job_finished(&mut self, job: &job::Job, readd_to_queue: bool) {
        if let Some(running) = &self.running {
            if &running.job == job {
                let finished_job = self.running.take().unwrap().job;
                if readd_to_queue {
                    tracing::debug!(id = %finished_job.id, label = ?finished_job.spec.label(), "adding job to front of queue again");
                    self.queue.push_front(finished_job);
                }
            }
        }
    }

    fn cancel(&mut self, reason: job::CancellationReason) {
        if let Some(send) = self.running.as_mut().and_then(|j| j.cancellation.take()) {
            if let Err(_) = send.send(reason) {
                tracing::warn!("cancellation receiver was dropped, job could not be cancelled");
            }
        }
    }
}

#[derive(Debug)]
struct PerRepositoryQueue {
    sender: events::Sender,
    restic: Arc<Restic>,
    secrets: Arc<Secrets>,
    repo_queue: RunQueue,
    per_backup_queues: HashMap<config::backup::Name, RunQueue>,
}

impl PerRepositoryQueue {
    fn new(sender: events::Sender, restic: Arc<Restic>, secrets: Arc<Secrets>) -> Self {
        let repo_queue = RunQueue::new(sender.clone(), restic.clone(), secrets.clone());
        PerRepositoryQueue {
            sender,
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
                    RunQueue::new(self.sender.clone(), restic.clone(), secrets.clone())
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

    fn job_finished(&mut self, job: &job::Job, readd_to_queue: bool) {
        self.repo_queue.job_finished(job, readd_to_queue);
        for queue in self.per_backup_queues.values_mut() {
            queue.job_finished(job, readd_to_queue);
        }
    }

    fn cancel(&mut self, reason: job::CancellationReason) {
        self.repo_queue.cancel(reason);
        for queue in self.per_backup_queues.values_mut() {
            queue.cancel(reason);
        }
    }

    fn has_running_jobs(&self) -> bool {
        self.repo_queue.has_running_job()
            || self.per_backup_queues.values().any(|q| q.has_running_job())
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
    per_repo_queues: HashMap<config::repo::Name, PerRepositoryQueue>,
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
            per_repo_queues: HashMap::new(),
        }
    }

    fn push(&mut self, job: job::Job) {
        self.per_repo_queues
            .entry(job.spec.queue_id().repo.clone())
            .or_insert_with(|| {
                PerRepositoryQueue::new(
                    self.events.sender.clone(),
                    self.restic.clone(),
                    self.secrets.clone(),
                )
            })
            .push(job);
    }

    fn maybe_start_next_jobs(&mut self) -> eyre::Result<()> {
        // start no jobs if suspended
        if self.suspend.is_suspended() {
            return Ok(());
        }
        // start more jobs as necessary
        for queue in self.per_repo_queues.values_mut() {
            queue.maybe_start_next_jobs()?;
        }
        Ok(())
    }

    fn job_finished(&mut self, job: &job::Job, readd_to_queue: bool) {
        for queue in self.per_repo_queues.values_mut() {
            queue.job_finished(job, readd_to_queue);
        }
    }

    fn cancel(&mut self, reason: job::CancellationReason) {
        for queue in self.per_repo_queues.values_mut() {
            queue.cancel(reason);
        }
    }

    fn has_running_jobs(&self) -> bool {
        self.per_repo_queues.values().any(|q| q.has_running_jobs())
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
            self.cancel(job::CancellationReason::Suspend);
        }
    }

    #[tracing::instrument(skip(self))]
    async fn handle_shutdown(&mut self, shutdown: ShutdownRequested) -> eyre::Result<()> {
        tracing::debug!("received shutdown event");
        self.cancel(job::CancellationReason::Shutdown);
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
            self.maybe_start_next_jobs()?;
        }
    }
}
