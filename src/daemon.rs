use cirrus_actor::Messages;
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
    #[allow(unused_variables)]
    let appconfig = Arc::new(appconfig);

    let (jobqueues_actor, jobqueues_ref) = cirrus_actor::new_actor();
    #[cfg(feature = "cirrus-desktop-ui")]
    let (desktop_ui_actor, desktop_ui_ref) = cirrus_actor::new_actor();

    let jobstatus_messages = Messages::new_discarding();
    #[cfg(feature = "cirrus-desktop-ui")]
    let jobstatus_messages = jobstatus_messages.also_to(desktop_ui_ref);

    let mut jobqueues_actor =
        jobqueues_actor.into_instance(job_queues::JobQueues::new(jobstatus_messages));

    #[cfg(feature = "cirrus-desktop-ui")]
    let mut desktop_ui_actor = desktop_ui_actor.into_instance(cirrus_desktop_ui::DesktopUi::new(
        appconfig.clone(),
        config.clone(),
        restic.clone(),
        secrets.clone(),
        jobqueues_ref.clone(),
    )?);

    let mut scheduler = scheduler::Scheduler::new(
        config.clone(),
        restic.clone(),
        secrets.clone(),
        jobqueues_ref.clone(),
    );

    let instance_name = hostname::get()?.to_string_lossy().into_owned();
    info!("instance name: {}", instance_name);

    tokio::spawn(async move { scheduler.run().await.unwrap() });
    tokio::spawn(async move { jobqueues_actor.run().await.unwrap() });
    #[cfg(feature = "cirrus-desktop-ui")]
    tokio::spawn(async move { desktop_ui_actor.run().await.unwrap() });

    info!("running forever...");
    tokio::signal::ctrl_c().await?;

    Ok(())
}
