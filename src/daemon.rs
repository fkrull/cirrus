use cirrus_core::{model::Config, restic::Restic, secrets::Secrets};
use cirrus_daemon::jobs::JobsRunner;
use cirrus_daemon::scheduler::Scheduler;
use cirrus_daemon::Daemon;
use clap::ArgMatches;
use log::info;
use std::sync::Arc;

pub async fn run(
    restic: Restic,
    secrets: Secrets,
    config: Config,
    _matches: &ArgMatches<'_>,
) -> eyre::Result<()> {
    let config = Arc::new(config);
    let restic = Arc::new(restic);
    let secrets = Arc::new(secrets);
    let (mut runner, jobs_ref) = JobsRunner::new();
    let mut scheduler = Scheduler::new(
        config.clone(),
        restic.clone(),
        secrets.clone(),
        jobs_ref.clone(),
    );

    let instance_name = hostname::get()?.to_string_lossy().into_owned();
    info!("instance name: {}", instance_name);
    let _daemon = Daemon {
        instance_name,
        config,
        restic,
        secrets,
        jobs_ref,
    };

    info!("starting job runner...");
    tokio::spawn(async move { runner.run().await.unwrap() });

    info!("starting scheduler...");
    tokio::spawn(async move { scheduler.run().await.unwrap() });

    info!("running forever...");
    tokio::signal::ctrl_c().await?;

    Ok(())
}
