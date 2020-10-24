use crate::job::{Job, JobStatus, JobStatusChange};
use cirrus_actor::ActorRef;
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
    job: Job,
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
    statuschange_sink: ActorRef<JobStatusChange>,
    running: Option<RunningJob>,
    queue: VecDeque<Job>,
}

impl RunQueue {
    fn new(statuschange_sink: ActorRef<JobStatusChange>) -> Self {
        RunQueue {
            statuschange_sink,
            running: None,
            queue: VecDeque::new(),
        }
    }

    fn push(&mut self, job: Job) {
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
                self.statuschange_sink
                    .send(JobStatusChange::new(job, JobStatus::Started))?;
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
                    JobStatus::FinishedSuccessfully
                }
                Err(error) => {
                    error!("job '{}' failed: {}", job.spec.name(), error);
                    JobStatus::FinishedWithError
                }
            };
            self.statuschange_sink
                .send(JobStatusChange::new(job, new_status))?;
        } else {
            futures::future::pending::<()>().await;
        }
        Ok(())
    }
}

#[derive(Debug)]
struct PerRepositoryQueue {
    statuschange_sink: ActorRef<JobStatusChange>,
    repo_queue: RunQueue,
    per_backup_queues: HashMap<model::backup::Name, RunQueue>,
}

impl PerRepositoryQueue {
    fn new(statuschange_sink: ActorRef<JobStatusChange>) -> Self {
        PerRepositoryQueue {
            repo_queue: RunQueue::new(statuschange_sink.clone()),
            statuschange_sink,
            per_backup_queues: HashMap::new(),
        }
    }

    fn push(&mut self, job: Job) {
        let sink = &self.statuschange_sink;
        match job.spec.queue_id().backup {
            Some(backup) => self
                .per_backup_queues
                .entry(backup.clone())
                .or_insert_with(|| RunQueue::new(sink.clone()))
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
            .map(|f| Box::pin(f));
        let backup_jobs = select_all_or_pending(backup_jobs);
        pin_mut!(backup_jobs);
        select(repo_job, backup_jobs).await;
        Ok(())
    }
}

#[derive(Debug)]
pub struct JobQueues {
    statuschange_sink: ActorRef<JobStatusChange>,
    per_repo_queues: HashMap<model::repo::Name, PerRepositoryQueue>,
}

impl JobQueues {
    pub fn new(statuschange_sink: ActorRef<JobStatusChange>) -> Self {
        JobQueues {
            statuschange_sink,
            per_repo_queues: HashMap::new(),
        }
    }

    fn push(&mut self, job: Job) {
        let sink = &self.statuschange_sink;
        self.per_repo_queues
            .entry(job.spec.queue_id().repo.clone())
            .or_insert_with(|| PerRepositoryQueue::new(sink.clone()))
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
            .map(|f| Box::pin(f));
        select_all_or_pending(jobs).await?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl cirrus_actor::Actor for JobQueues {
    type Message = Job;
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
