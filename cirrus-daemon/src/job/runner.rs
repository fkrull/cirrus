use crate::job;
use cirrus_core::{
    restic::{Options, Output, Restic, Verbosity},
    secrets::Secrets,
};
use std::{sync::Arc, time::Duration};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    sync::oneshot,
};

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
                self.sender.send(job::StatusChange::new(
                    job,
                    job::Status::Cancelled(cancellation_reason),
                ));
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
            stdout: Output::Capture,
            stderr: Output::Capture,
            verbose: Verbosity::V,
            ..Default::default()
        },
    )?;

    let mut stdout = BufReader::new(
        process
            .stdout()
            .take()
            .expect("should be present based on params"),
    )
    .lines();
    let mut stderr = BufReader::new(
        process
            .stderr()
            .take()
            .expect("should be present based on params"),
    )
    .lines();

    loop {
        // TODO: use JSON output, process into better updates
        tokio::select! {
            line = stdout.next_line() => match line? {
                Some(line) => tracing::info!("{}", line),
                None => break,
            },
            line = stderr.next_line() => match line? {
                Some(line) => tracing::warn!("{}", line),
                None => break,
            },
            cancellation_reason = &mut cancellation => {
                process.terminate(TERMINATE_GRACE_PERIOD).await?;
                return Ok(Err(cancellation_reason?));
            }
        }
    }

    Ok(Ok(process.check_wait().await?))
}
