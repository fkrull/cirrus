use cirrus_actor::ActorInstance;
use cirrus_core::{model::Config, restic::Restic, secrets::Secrets};
use cirrus_daemon::{job_queues, scheduler, Daemon};
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
    let (mut retry_handler_actor, retry_handler) =
        ActorInstance::new(cirrus_actor::NullSink::new());
    let (mut job_queues_actor, job_queues) =
        ActorInstance::new(job_queues::JobQueues::new(retry_handler.clone()));
    let mut scheduler = scheduler::Scheduler::new(
        config.clone(),
        restic.clone(),
        secrets.clone(),
        job_queues.clone(),
    );

    let instance_name = hostname::get()?.to_string_lossy().into_owned();
    info!("instance name: {}", instance_name);
    let _daemon = Daemon {
        instance_name,
        config,
        restic,
        secrets,
        job_queues,
        retry_handler,
    };

    tokio::spawn(async move { job_queues_actor.run().await.unwrap() });
    tokio::spawn(async move { scheduler.run().await.unwrap() });
    tokio::spawn(async move { retry_handler_actor.run().await.unwrap() });

    info!("running forever...");
    tokio::signal::ctrl_c().await?;

    Ok(())
}
