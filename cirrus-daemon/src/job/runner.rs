use crate::job;
use cirrus_core::{
    restic::{Event, Options, Restic, Verbosity},
    secrets::Secrets,
};
use futures::StreamExt;
use shindig::Events;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub(super) struct Cancel {
    pub id: job::Id,
    pub reason: job::CancellationReason,
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
    pub(super) async fn run(
        &mut self,
        job: job::Job,
        mut cancel_recv: shindig::Subscriber<Cancel>,
    ) {
        self.events
            .send(job::StatusChange::new(job.clone(), job::Status::Started));
        let run_future = run(job.spec.clone(), self.restic.clone(), self.secrets.clone());
        tokio::pin!(run_future);
        loop {
            tokio::select! {
                result = &mut run_future => {
                    self.handle_result(result, &job);
                    break;
                }
                cancel = cancel_recv.recv() => {
                    if let Ok(cancel) = cancel {
                        if let Some(_) = self.handle_cancel(cancel, &job) {
                            break;
                        }
                    }
                }
            }
        }
    }

    fn handle_result(&mut self, result: eyre::Result<()>, job: &job::Job) {
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

    fn handle_cancel(&mut self, cancel: Cancel, job: &job::Job) -> Option<job::CancellationReason> {
        if cancel.id == job.id {
            tracing::info!(reason = ?cancel.reason, "cancelled");
            self.events
                .send(job::StatusChange::new(job.clone(), job::Status::Cancelled));
            Some(cancel.reason)
        } else {
            None
        }
    }
}

async fn run(spec: job::Spec, restic: Arc<Restic>, secrets: Arc<Secrets>) -> eyre::Result<()> {
    match spec {
        job::Spec::Backup(backup_spec) => run_backup(&backup_spec, &restic, &secrets).await,
    }
}

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
