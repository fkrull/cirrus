use crate::{
    jobs::{repo::JobsRepo, runner::JobsRunner},
    model::Config,
    restic::Restic,
    secrets::Secrets,
};
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
    tokio::spawn(async move { runner.run_jobs().await });

    info!("running cirrus daemon...");
    futures::future::pending::<()>().await;
    Ok(())
}
