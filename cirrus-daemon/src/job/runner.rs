use crate::job;
use cirrus_core::{
    restic::{Event, Options, Restic, Verbosity},
    secrets::Secrets,
};
use futures::StreamExt;
use shindig::Events;
use std::sync::Arc;

async fn run_backup(
    spec: &job::BackupSpec,
    restic: &Restic,
    secrets: &Secrets,
) -> eyre::Result<()> {
    let repo_with_secrets = secrets.get_secrets(&spec.repo)?;
    let mut process = restic.backup(
        &repo_with_secrets,
        &spec.backup_name,
        &spec.backup,
        &Options {
            capture_output: true,
            verbose: Verbosity::V,
            ..Default::default()
        },
    )?;

    while let Some(event) = process.next().await {
        // TODO: use JSON output, process into better updates
        match event? {
            Event::StdoutLine(line) => {
                tracing::info!("{}", line);
            }
            Event::StderrLine(line) => {
                tracing::warn!("{}", line);
            }
        }
    }

    Ok(process.check_wait().await?)
}

#[derive(Debug)]
pub(super) struct Runner {
    events: Events,
    restic: Arc<Restic>,
    secrets: Arc<Secrets>,
}

impl Runner {
    pub(super) fn new(events: Events, restic: Arc<Restic>, secrets: Arc<Secrets>) -> Self {
        Runner {
            events,
            restic,
            secrets,
        }
    }

    #[tracing::instrument(name = "job", skip_all, fields(id = %job.id, label = job.spec.label()))]
    pub(super) async fn run(&mut self, job: job::Job) {
        self.events
            .send(job::StatusChange::new(job.clone(), job::Status::Started));
        let result = match &job.spec {
            job::Spec::Backup(backup_spec) => {
                run_backup(backup_spec, &self.restic, &self.secrets).await
            }
        };
        match result {
            Ok(_) => {
                tracing::info!("finished successfully");
                self.events.send(job::StatusChange::new(
                    job.clone(),
                    job::Status::FinishedSuccessfully,
                ));
            }
            Err(error) => {
                tracing::error!(%error, "failed");
                self.events.send(job::StatusChange::new(
                    job.clone(),
                    job::Status::FinishedWithError,
                ));
            }
        }
    }
}
