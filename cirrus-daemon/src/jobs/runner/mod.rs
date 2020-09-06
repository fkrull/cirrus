use super::*;
use crate::jobs::repo::JobsRepo;
use crate::jobs::runner::backup::run_backup_job;
use cirrus_core::{restic::Restic, secrets::Secrets};
use futures::{future::Either, pin_mut, prelude::*};
use log::{error, info};
use std::{fmt::Debug, future::Future, pin::Pin, sync::Arc};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

mod backup;

trait RunningJob: Debug + Send {
    fn next(&mut self) -> Pin<Box<dyn Future<Output = Job> + Send + '_>>;
}

#[derive(Debug)]
enum JobsRunnerSelect {
    UpdatedJob { job: Job, running_job_idx: usize },
    EnqueuedJob(JobDescription),
    EndOfQueue,
}

#[derive(Debug)]
pub struct JobsRunner {
    restic: Arc<Restic>,
    secrets: Arc<Secrets>,
    jobs_repo: Arc<JobsRepo>,
    recv: UnboundedReceiver<JobDescription>,
    running_jobs: Vec<Box<dyn RunningJob>>,
}

impl JobsRunner {
    pub fn new(
        restic: Arc<Restic>,
        secrets: Arc<Secrets>,
        jobs_repo: Arc<JobsRepo>,
    ) -> (JobsRunner, JobsRunnerSender) {
        let (send, recv) = unbounded_channel();
        let runner = JobsRunner {
            restic,
            secrets,
            jobs_repo,
            recv,
            running_jobs: Vec::new(),
        };
        (runner, JobsRunnerSender(send))
    }

    pub async fn run_jobs(&mut self) {
        loop {
            match self.select().await {
                JobsRunnerSelect::UpdatedJob {
                    job,
                    running_job_idx,
                } => self.update_job(job, running_job_idx).await,
                JobsRunnerSelect::EnqueuedJob(desc) => self.spawn_job(desc).await,
                JobsRunnerSelect::EndOfQueue => {
                    info!("stopping job runner because all send ends were closed");
                    break;
                }
            };
        }
    }

    async fn select(&mut self) -> JobsRunnerSelect {
        if self.running_jobs.is_empty() {
            match self.recv.recv().await {
                Some(desc) => JobsRunnerSelect::EnqueuedJob(desc),
                None => JobsRunnerSelect::EndOfQueue,
            }
        } else {
            let job_updates = future::select_all(self.running_jobs.iter_mut().map(|x| x.next()));
            pin_mut!(job_updates);
            let recv = self.recv.recv();
            pin_mut!(recv);

            match future::select(job_updates, recv).await {
                Either::Left(((job, running_job_idx, _), _)) => JobsRunnerSelect::UpdatedJob {
                    job,
                    running_job_idx,
                },
                Either::Right((Some(desc), _)) => JobsRunnerSelect::EnqueuedJob(desc),
                Either::Right((None, _)) => JobsRunnerSelect::EndOfQueue,
            }
        }
    }

    async fn update_job(&mut self, job: Job, running_job_idx: usize) {
        if job.is_finished() {
            self.running_jobs.remove(running_job_idx);
        }
        self.jobs_repo.save(job).await;
    }

    async fn spawn_job(&mut self, description: JobDescription) {
        let mut job = Job {
            id: self.jobs_repo.next_id(),
            description: description.clone(),
            status: JobStatus::Running,
            started: cirrus_core::timestamp::now(),
            finished: None,
        };

        let result = match description {
            JobDescription::Backup { backup, repo, .. } => {
                run_backup_job(&self.restic, &self.secrets, backup, repo, &job)
            }
        };
        match result {
            Ok(running_job) => {
                self.running_jobs.push(running_job);
            }
            Err(err) => {
                error!("job failed to start: {}", err);
                job.status = JobStatus::FailedToStart;
            }
        }

        self.jobs_repo.save(job).await;
    }
}

#[derive(Debug, Clone)]
pub struct JobsRunnerSender(UnboundedSender<JobDescription>);

impl JobsRunnerSender {
    pub fn enqueue(&self, desc: JobDescription) {
        if let Err(err) = self.0.send(desc) {
            error!(
                "enqueuing a job failed (was the job runner shut down?): {}",
                err
            );
        }
    }
}
