use crate::{
    jobs::{repo::JobsRepo, Job, JobDescription},
    restic::Restic,
    secrets::Secrets,
};
use futures::{future::select_all, prelude::*, select};
use log::{info, warn};
use std::{fmt::Debug, future::Future, sync::Arc};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

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
            select! {
                (job, idx, _) = select_all(self.running_jobs.iter_mut().map(|x| x.next())).fuse() => {
                    if job.is_finished() {
                        self.running_jobs.remove(idx);
                    }
                    self.jobs_repo.save(job).await;
                }
                maybe_desc = self.recv.recv().fuse() => match maybe_desc {
                    Some(desc) => {
                        match desc {
                            JobDescription::Backup { definition } => {
                                // TODO: also save job
                                todo!()
                            }
                        }
                    },
                    None => {
                        info!("stopping job runner because all send ends were closed");
                        break;
                    }
                }
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
