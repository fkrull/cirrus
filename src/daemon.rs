use cirrus_core::appconfig::AppConfig;
use cirrus_core::{model::Config, restic::Restic, secrets::Secrets};
use cirrus_daemon::*;
use clap::ArgMatches;
use log::info;
use std::sync::Arc;

pub async fn run(
    restic: Restic,
    secrets: Secrets,
    config: Config,
    appconfig: AppConfig,
    _matches: &ArgMatches<'_>,
) -> eyre::Result<()> {
    let restic = Arc::new(restic);
    let secrets = Arc::new(secrets);
    let config = Arc::new(config);
    let appconfig = Arc::new(appconfig);

    let (jobqueues_actor, jobqueues) = cirrus_actor::new_actor();
    let (retryhandler_actor, retryhandler) = cirrus_actor::new_actor();
    let (multiplexer_actor, multiplexer) = cirrus_actor::new_actor();
    #[cfg(feature = "cirrus-desktop-ui")]
    let (desktop_ui_actor, desktop_ui) = cirrus_actor::new_actor();
    let (jobhistory_actor, jobhistory) = cirrus_actor::new_actor();

    let mut jobqueues_actor =
        jobqueues_actor.into_instance(job_queues::JobQueues::new(retryhandler));
    let mut retryhandler_actor =
        retryhandler_actor.into_instance(retry::RetryHandler::new(jobqueues.clone(), multiplexer));
    let mut multiplexer_actor =
        multiplexer_actor.into_instance(cirrus_actor::util::MultiplexActor::new_with([
            #[cfg(feature = "cirrus-desktop-ui")]
            desktop_ui,
            jobhistory,
        ]));
    #[cfg(feature = "cirrus-desktop-ui")]
    let mut desktop_ui_actor =
        desktop_ui_actor.into_instance(cirrus_desktop_ui::DesktopUi::new(appconfig.clone())?);
    let mut jobhistory_actor = jobhistory_actor.into_instance(cirrus_actor::util::NullSink::new());

    let mut scheduler = scheduler::Scheduler::new(
        config.clone(),
        restic.clone(),
        secrets.clone(),
        jobqueues.clone(),
    );

    let instance_name = hostname::get()?.to_string_lossy().into_owned();
    info!("instance name: {}", instance_name);

    tokio::spawn(async move { scheduler.run().await.unwrap() });
    tokio::spawn(async move { jobqueues_actor.run().await.unwrap() });
    tokio::spawn(async move { retryhandler_actor.run().await.unwrap() });
    tokio::spawn(async move { multiplexer_actor.run().await.unwrap() });
    #[cfg(feature = "cirrus-desktop-ui")]
    tokio::spawn(async move { desktop_ui_actor.run().await.unwrap() });
    tokio::spawn(async move { jobhistory_actor.run().await.unwrap() });

    info!("running forever...");
    tokio::signal::ctrl_c().await?;

    Ok(())
}
