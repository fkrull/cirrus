use crate::job;
use cirrus_core::{
    restic::{Event, Options, Restic, Verbosity},
    secrets::Secrets,
};
use futures::StreamExt;
use std::{sync::Arc, time::Duration};
use tokio::sync::oneshot;

#[derive(Debug)]
pub(super) struct Runner {
    sender: events::Sender,
    restic: Arc<Restic>,
    secrets: Arc<Secrets>,
}

impl Runner {
    pub(super) fn new(sender: events::Sender, restic: Arc<Restic>, secrets: Arc<Secrets>) -> Self {
        Runner {
            sender,
            restic,
            secrets,
        }
    }

    #[tracing::instrument(name = "job", skip_all, fields(id = %job.id, label = job.spec.label()))]
    pub(super) async fn run(
        &mut self,
        job: job::Job,
        cancellation: oneshot::Receiver<job::CancellationReason>,
    ) {
        self.sender
            .send(job::StatusChange::new(job.clone(), job::Status::Started));
        let run_result = run(
            job.spec.clone(),
            self.restic.clone(),
            self.secrets.clone(),
            cancellation,
        )
        .await;
        match run_result {
            Ok(Ok(_)) => {
                tracing::info!("finished successfully");
                self.sender.send(job::StatusChange::new(
                    job,
                    job::Status::FinishedSuccessfully,
                ));
            }
            Ok(Err(cancellation_reason)) => {
                tracing::info!(reason = ?cancellation_reason, "cancelled");
                self.sender
                    .send(job::StatusChange::new(job, job::Status::Cancelled));
            }
            Err(error) => {
                tracing::error!(%error, "failed");
                self.sender
                    .send(job::StatusChange::new(job, job::Status::FinishedWithError));
            }
        }
    }
}

async fn run(
    spec: job::Spec,
    restic: Arc<Restic>,
    secrets: Arc<Secrets>,
    cancellation: oneshot::Receiver<job::CancellationReason>,
) -> eyre::Result<Result<(), job::CancellationReason>> {
    match spec {
        job::Spec::Backup(backup_spec) => {
            run_backup(&backup_spec, &restic, &secrets, cancellation).await
        }
    }
}

const TERMINATE_GRACE_PERIOD: Duration = Duration::from_secs(2);

async fn run_backup(
    spec: &job::BackupSpec,
    restic: &Restic,
    secrets: &Secrets,
    mut cancellation: oneshot::Receiver<job::CancellationReason>,
) -> eyre::Result<Result<(), job::CancellationReason>> {
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

    // TODO: more thoroughly guarantee that the process is terminated even on error returns
    loop {
        tokio::select! {
            event = process.next() => {
                match event {
                    // TODO: use JSON output, process into better updates
                    Some(Ok(Event::StdoutLine(line))) => tracing::info!("{}", line),
                    Some(Ok(Event::StderrLine(line))) => tracing::warn!("{}", line),
                    Some(Err(error)) => return Err(error.into()),
                    None => break
                }
            },
            cancellation_reason = &mut cancellation => {
                process.terminate(TERMINATE_GRACE_PERIOD).await?;
                return Ok(Err(cancellation_reason?));
            }
        }
    }

    Ok(Ok(process.check_wait().await?))
}
