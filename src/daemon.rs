use cirrus_core::{model::Config, restic::Restic, secrets::Secrets};
use cirrus_daemon::{job_queues, retry, scheduler};
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

    let (jobhistory_actor, jobhistory) = cirrus_actor::new_actor();
    let (retryhandler_actor, retryhandler) = cirrus_actor::new_actor();
    let (jobqueues_actor, jobqueues) = cirrus_actor::new_actor();

    let mut jobhistory_actor = jobhistory_actor.into_instance(cirrus_actor::util::NullSink::new());
    let mut retryhandler_actor =
        retryhandler_actor.into_instance(retry::RetryHandler::new(jobqueues.clone(), jobhistory));
    let mut jobqueues_actor =
        jobqueues_actor.into_instance(job_queues::JobQueues::new(retryhandler));

    let mut scheduler = scheduler::Scheduler::new(
        config.clone(),
        restic.clone(),
        secrets.clone(),
        jobqueues.clone(),
    );

    let instance_name = hostname::get()?.to_string_lossy().into_owned();
    info!("instance name: {}", instance_name);

    tokio::spawn(async move { jobqueues_actor.run().await.unwrap() });
    tokio::spawn(async move { scheduler.run().await.unwrap() });
    tokio::spawn(async move { retryhandler_actor.run().await.unwrap() });
    tokio::spawn(async move { jobhistory_actor.run().await.unwrap() });

    info!("running forever...");
    tokio::signal::ctrl_c().await?;

    Ok(())
}
