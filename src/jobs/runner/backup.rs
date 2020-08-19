use crate::jobs::runner::RunningJob;
use crate::jobs::Job;
use crate::model::backup;
use crate::restic::Restic;
use crate::secrets::Secrets;
use futures::Future;

#[derive(Debug)]
struct BackupJob {}

impl BackupJob {}

impl RunningJob for BackupJob {
    fn next(&mut self) -> Box<dyn Future<Output = Job> + Unpin + Send> {
        todo!()
    }
}

pub(super) fn run_backup_job(
    restic: &Restic,
    secrets: &Secrets,
    backup: backup::Definition,
    job: &Job,
) -> anyhow::Result<Box<dyn RunningJob>> {
    todo!();
    Ok(Box::new(BackupJob {}))
}
