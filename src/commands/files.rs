use crate::cli::files::{Cli, Cmd, Index};
use cirrus_core::{
    cache::Cache,
    config::{repo, Config},
    restic::Restic,
    secrets::Secrets,
};
use cirrus_daemon::{job::Status, shutdown::RequestShutdown, suspend::Suspend};
use std::sync::Arc;

pub async fn main(
    restic: Restic,
    secrets: Secrets,
    cache: Cache,
    config: Config,
    args: Cli,
) -> eyre::Result<()> {
    let repo_name = repo::Name(args.repository);
    let repo = config
        .repositories
        .get(&repo_name)
        .ok_or_else(|| eyre::eyre!("unknown repository {}", repo_name.0))?;
    match args.subcommand {
        Cmd::Index(args) => update(restic, secrets, cache, repo_name, repo.clone(), args).await,
    }
}

async fn update(
    restic: Restic,
    secrets: Secrets,
    cache: Cache,
    repo_name: repo::Name,
    repo: repo::Definition,
    args: Index,
) -> eyre::Result<()> {
    let restic = Arc::new(restic);
    let secrets = Arc::new(secrets);
    let mut events = events::Builder::new_with_capacity(128);

    let mut job_queues = cirrus_daemon::job::queues::JobQueues::new(
        &mut events,
        restic.clone(),
        secrets.clone(),
        cache.clone(),
        Suspend::NotSuspended,
    );
    let mut shutdown_service = cirrus_daemon::shutdown::ShutdownService::new(&mut events);
    let mut signal_handler = cirrus_daemon::signal_handler::SignalHandler::new(&mut events);
    let mut status_changes = events.subscribe::<cirrus_daemon::job::StatusChange>();

    tokio::spawn(async move { job_queues.run().await.unwrap() });
    tokio::spawn(async move { shutdown_service.run().await.unwrap() });
    tokio::spawn(async move { signal_handler.run().await.unwrap() });

    let spec = cirrus_daemon::job::FilesIndexSpec {
        repo_name,
        repo,
        max_age: args.max_age,
    };
    let job = cirrus_daemon::job::Job::new(spec.into());
    let id = job.id;
    events.typed_sender::<cirrus_daemon::job::Job>().send(job);

    loop {
        let status_change = status_changes.recv().await?;
        if status_change.job.id == id {
            match status_change.new_status {
                Status::Cancelled(_) | Status::FinishedSuccessfully | Status::FinishedWithError => {
                    // TODO what do we do?!?!?!!!
                    events
                        .typed_sender::<RequestShutdown>()
                        .send(RequestShutdown);
                    break;
                }
                Status::Started => (),
            }
        }
    }

    // TODO: not pretty, can I avoid that?
    futures::future::pending::<eyre::Result<()>>().await
}
