use crate::job;
use cirrus_core::{
    cache::Cache,
    restic::{Options, Output, Restic, Verbosity},
    secrets::Secrets,
};
use std::{sync::Arc, time::Duration};
use time::OffsetDateTime;
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    sync::oneshot,
};

#[derive(Debug)]
pub(super) struct Runner {
    sender: events::Sender,
    restic: Arc<Restic>,
    secrets: Arc<Secrets>,
    cache: Cache,
}

impl Runner {
    pub(super) fn new(
        sender: events::Sender,
        restic: Arc<Restic>,
        secrets: Arc<Secrets>,
        cache: Cache,
    ) -> Self {
        Runner {
            sender,
            restic,
            secrets,
            cache,
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
            self.cache.clone(),
            cancellation,
        )
        .await;
        match run_result {
            Ok(_) => {
                tracing::info!("finished successfully");
                self.sender.send(job::StatusChange::new(
                    job,
                    job::Status::FinishedSuccessfully,
                ));
            }
            Err(JobOutcome::Cancelled(cancellation_reason)) => {
                tracing::info!(reason = ?cancellation_reason, "cancelled");
                self.sender.send(job::StatusChange::new(
                    job,
                    job::Status::Cancelled(cancellation_reason),
                ));
            }
            Err(JobOutcome::Error(error)) => {
                tracing::error!(%error, "failed");
                self.sender
                    .send(job::StatusChange::new(job, job::Status::FinishedWithError));
            }
        }
    }
}

#[derive(Debug)]
enum JobOutcome {
    Error(eyre::Report),
    Cancelled(job::CancellationReason),
}

impl<E: Into<eyre::Report>> From<E> for JobOutcome {
    fn from(e: E) -> Self {
        JobOutcome::Error(e.into())
    }
}

impl From<job::CancellationReason> for JobOutcome {
    fn from(r: job::CancellationReason) -> Self {
        JobOutcome::Cancelled(r)
    }
}

async fn run(
    spec: job::Spec,
    restic: Arc<Restic>,
    secrets: Arc<Secrets>,
    cache: Cache,
    cancellation: oneshot::Receiver<job::CancellationReason>,
) -> Result<(), JobOutcome> {
    match spec {
        job::Spec::Backup(spec) => run_backup(&spec, &restic, &secrets, cancellation).await,
        job::Spec::FilesIndex(spec) => {
            update_files_index(&spec, &restic, &secrets, &cache, cancellation).await
        }
    }
}

const TERMINATE_GRACE_PERIOD: Duration = Duration::from_secs(5);

async fn run_backup(
    spec: &job::BackupSpec,
    restic: &Restic,
    secrets: &Secrets,
    mut cancellation: oneshot::Receiver<job::CancellationReason>,
) -> Result<(), JobOutcome> {
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
                return Err(cancellation_reason?.into());
            }
        }
    }

    Ok(process.check_wait().await?)
}

fn check_cancellation(
    cancellation: &mut oneshot::Receiver<job::CancellationReason>,
) -> Result<(), JobOutcome> {
    match cancellation.try_recv() {
        Err(oneshot::error::TryRecvError::Empty) => Ok(()),
        Ok(cancellation_reason) => Err(cancellation_reason.into()),
        Err(err) => Err(err.into()),
    }
}

async fn update_files_index(
    spec: &job::FilesIndexSpec,
    restic: &Restic,
    secrets: &Secrets,
    cache: &Cache,
    mut cancellation: oneshot::Receiver<job::CancellationReason>,
) -> Result<(), JobOutcome> {
    tracing::info!(target: "cli", "Indexing snapshots...");

    // update snapshots
    let mut db = cirrus_index::Database::new(cache.get().await?, &spec.repo_name).await?;
    let repo_with_secrets = secrets.get_secrets(&spec.repo)?;
    let num_snapshots = cirrus_index::index_snapshots(restic, &mut db, &repo_with_secrets).await?;
    tracing::info!(target: "cli", "{num_snapshots} snapshots in repository.");

    check_cancellation(&mut cancellation)?;

    if let Some(max_age) = spec.max_age.or(spec.repo.build_index) {
        let newer_than = OffsetDateTime::now_utc() - max_age;
        let snapshots = db.get_unindexed_snapshots(newer_than).await?;
        tracing::info!(
            target: "cli",
            "Indexing snapshot contents for the past {} ({} snapshots)...",
            humantime::format_duration(max_age),
            snapshots.len()
        );
        check_cancellation(&mut cancellation)?;
        for snapshot in &snapshots {
            tracing::info!(target: "cli", "Indexing {}...", snapshot.short_id());
            // TODO fix async handling and reenable
            //cirrus_index::index_files(restic, &mut db, &repo_with_secrets, snapshot).await?;
            check_cancellation(&mut cancellation)?;
        }
        tracing::info!(target: "cli", "Finished indexing snapshot contents ({} snapshots).", snapshots.len());
    } else {
        tracing::info!(target: "cli", "Not indexing any snapshot contents.");
    }

    Ok(())
}
