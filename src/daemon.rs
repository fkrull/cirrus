use cirrus_core::{model::Config, restic::Restic, secrets::Secrets};
use cirrus_daemon::jobs::{repo::JobsRepo, runner::JobsRunner};
use clap::ArgMatches;
use log::info;
use std::sync::Arc;

pub async fn run(
    restic: Restic,
    secrets: Secrets,
    config: Config,
    _matches: &ArgMatches<'_>,
) -> anyhow::Result<()> {
    let _config = Arc::new(config);
    let restic = Arc::new(restic);
    let secrets = Arc::new(secrets);
    let jobs_repo = Arc::new(JobsRepo::new());
    let (mut runner, _sender) = JobsRunner::new(restic.clone(), secrets.clone(), jobs_repo.clone());

    info!("starting job runner...");
    tokio::spawn(async move { runner.run_jobs().await });

    info!("starting web UI...");
    cirrus_web::launch().await?;
    Ok(())
}