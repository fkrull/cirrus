use crate::jobs::runner::backup::run_backup_job;
use crate::jobs::JobStatus;
use crate::{
    jobs::{repo::JobsRepo, Job, JobDescription},
    restic::Restic,
    secrets::Secrets,
};
use futures::{future::select_all, prelude::*, select};
use log::{error, info, warn};
use std::{fmt::Debug, future::Future, sync::Arc};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

mod backup;

trait RunningJob: Debug + Send {
    fn next(&mut self) -> Box<dyn Future<Output = Job> + Unpin + Send>;
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
            let job_updates = self.running_jobs.iter_mut().map(|x| x.next());

            select! {
                (job, idx, _) = select_all(job_updates).fuse() => {
                    if job.is_finished() {
                        self.running_jobs.remove(idx);
                    }
                    self.jobs_repo.save(job).await;
                }
                maybe_desc = self.recv.recv().fuse() => match maybe_desc {
                    Some(desc) => self.spawn_job(desc).await,
                    None => {
                        info!("stopping job runner because all send ends were closed");
                        break;
                    }
                }
            }
        }
    }

    async fn spawn_job(&mut self, description: JobDescription) {
        let job = Job {
            id: self.jobs_repo.next_id(),
            description: description.clone(),
            status: JobStatus::Running,
            started: crate::timestamp::now(),
            finished: None,
        };

        let result = match description {
            JobDescription::Backup { definition } => {
                run_backup_job(&self.restic, &self.secrets, definition, &job)
            }
        };
        match result {
            Ok(running_job) => {
                self.running_jobs.push(running_job);
                self.jobs_repo.save(job).await;
            }
            Err(err) => {
                error!("job failed to start: {}", err);
                // TODO: any other reporting?
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct JobsRunnerSender(UnboundedSender<JobDescription>);

impl JobsRunnerSender {
    pub fn enqueue(&self, desc: JobDescription) {
        if let Err(err) = self.0.send(desc) {
            warn!(
                "enqueuing a job failed (was the job runner shut down?): {}",
                err
            );
        }
    }
}
