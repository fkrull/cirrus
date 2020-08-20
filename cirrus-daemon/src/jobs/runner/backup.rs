use crate::jobs::{runner::RunningJob, Job, JobStatus};
use cirrus_core::{
    model::{backup, repo},
    restic::{Event, Options, Restic, ResticProcess},
    secrets::Secrets,
};
use futures::Future;
use log::warn;
use std::pin::Pin;

#[derive(Debug)]
struct BackupJob {
    process: ResticProcess,
    job: Job,
}

impl BackupJob {
    async fn handle_events(&mut self) -> anyhow::Result<Job> {
        loop {
            let event = self.process.next_event().await?;
            if let Event::ProcessExit(exit_status) = event {
                if !exit_status.success() {
                    self.job.finish(JobStatus::Error);
                } else {
                    self.job.finish(JobStatus::Error);
                }
                return Ok(self.job.clone());
            }
        }
    }
}

impl RunningJob for BackupJob {
    fn next(&mut self) -> Pin<Box<dyn Future<Output = Job> + Send + '_>> {
        Box::pin(async move {
            match self.handle_events().await {
                Ok(job) => job,
                Err(err) => {
                    warn!("backup job failed with internal error: {}", err);
                    self.process.kill();
                    self.job.finish(JobStatus::InternalError);
                    self.job.clone()
                }
            }
        })
    }
}

pub(super) fn run_backup_job(
    restic: &Restic,
    secrets: &Secrets,
    backup: backup::Definition,
    repo: repo::Definition,
    job: &Job,
) -> anyhow::Result<Box<dyn RunningJob>> {
    let repo_with_secrets = secrets.get_secrets(&repo)?;
    let process = restic.backup(
        repo_with_secrets,
        &backup,
        &Options {
            capture_output: false,
            ..Default::default()
        },
    )?;
    Ok(Box::new(BackupJob {
        process,
        job: job.clone(),
    }))
}
