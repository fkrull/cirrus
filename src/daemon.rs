use cirrus_core::{model::Config, restic::Restic, secrets::Secrets};
use cirrus_daemon::actor::ActorInstance;
use cirrus_daemon::jobs::JobsRunner;
use cirrus_daemon::Daemon;
use clap::ArgMatches;
use log::info;
use std::sync::Arc;

pub async fn run(
    restic: Restic,
    secrets: Secrets,
    config: Config,
    _matches: &ArgMatches<'_>,
) -> anyhow::Result<()> {
    let config = Arc::new(config);
    let restic = Arc::new(restic);
    let secrets = Arc::new(secrets);
    let (mut runner, jobs_ref) = ActorInstance::new(JobsRunner::default());

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

    info!("running forever...");
    tokio::signal::ctrl_c().await?;

    Ok(())
}
