use crate::{
    model::{backup, Config},
    restic::Restic,
    secrets::Secrets,
    Timestamp,
};
use futures::{future::select_all, prelude::*, select};
use log::{info, warn};
use std::{fmt::Debug, future::Future, sync::Arc};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum JobDescription {
    Backup { definition: backup::Definition },
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum JobStatus {
    Running,
    Successful,
    Error,
}

impl JobStatus {
    fn is_running(&self) -> bool {
        match self {
            JobStatus::Running => true,
            _ => false,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Job {
    pub id: u64,
    pub description: JobDescription,
    pub status: JobStatus,
    pub started: Timestamp,
    pub finished: Option<Timestamp>,
}

impl Job {
    fn is_finished(&self) -> bool {
        !self.status.is_running()
    }
}

trait RunningJob: Debug {
    fn next(&mut self) -> Box<dyn Future<Output = Job> + Unpin>;
}

#[derive(Debug)]
pub struct Jobs {
    restic: Arc<Restic>,
    secrets: Arc<Secrets>,

    recv: UnboundedReceiver<JobDescription>,
    running_jobs: Vec<Box<dyn RunningJob>>,
    jobs: Vec<Job>,
}

impl Jobs {
    pub fn new(restic: Arc<Restic>, secrets: Arc<Secrets>) -> (Jobs, JobsQueue) {
        let (send, recv) = unbounded_channel();
        let jobs = Jobs {
            restic,
            secrets,

            recv,
            running_jobs: Vec::new(),
            jobs: Vec::new(),
        };
        let jobs_queue = JobsQueue(send);
        (jobs, jobs_queue)
    }

    // TODO: get jobs out

    pub async fn run_jobs(&mut self) {
        //while let Some(desc) = self.recv.recv().await {}
        loop {
            select! {
                (job, idx, _) = select_all(self.running_jobs.iter_mut().map(|x| x.next())).fuse() => {
                    if job.is_finished() {
                        self.running_jobs.remove(idx);
                    }
                    // TODO: externalize somewhere else
                    let idx = job.id as usize;
                    self.jobs[idx] = job;
                }
                maybe_desc = self.recv.recv().fuse() => match maybe_desc {
                    Some(desc) => {
                        match desc {
                            JobDescription::Backup { definition } => {
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
pub struct JobsQueue(UnboundedSender<JobDescription>);

impl JobsQueue {
    pub fn enqueue(&self, desc: JobDescription) {
        if let Err(err) = self.0.send(desc) {
            warn!(
                "enqueuing a job failed (was the job runner shut down?): {}",
                err
            );
        }
    }
}
