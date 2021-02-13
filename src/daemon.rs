use cirrus_actor::Messages;
use cirrus_core::{model::Config, restic::Restic, secrets::Secrets};
use cirrus_daemon::*;
use log::info;
use std::sync::Arc;

pub async fn run(restic: Restic, secrets: Secrets, config: Config) -> eyre::Result<()> {
    let restic = Arc::new(restic);
    let secrets = Arc::new(secrets);
    let config = Arc::new(config);
    #[allow(unused_variables)]
    let daemon_config = Arc::new(daemon_config::DaemonConfig::default());

    // declare actors
    let jobqueues = cirrus_actor::new();
    let scheduler = cirrus_actor::new();
    let configreloader = cirrus_actor::new();
    #[cfg(feature = "cirrus-desktop-ui")]
    let desktop_ui = cirrus_actor::new();

    // connect multicast
    let jobstatus_sink = Messages::default();
    #[cfg(feature = "cirrus-desktop-ui")]
    let jobstatus_sink = jobstatus_sink.also_to(desktop_ui.actor_ref());

    let configreload_sink = Messages::default().also_to(scheduler.actor_ref());
    #[cfg(feature = "cirrus-desktop-ui")]
    let configreload_sink = configreload_sink.also_to(desktop_ui.actor_ref());

    // create actor instances
    let jobqueues_ref = jobqueues.actor_ref();
    let mut jobqueues = jobqueues.into_instance(job_queues::JobQueues::new(
        jobstatus_sink,
        restic.clone(),
        secrets.clone(),
    ));

    let mut scheduler = scheduler.into_instance(scheduler::Scheduler::new(
        config.clone(),
        jobqueues_ref.clone(),
    ));

    let configreloader_ref = configreloader.actor_ref();
    let mut configreloader = configreloader.into_instance(configreload::ConfigReloader::new(
        config.clone(),
        configreloader_ref,
        configreload_sink,
    )?);

    #[cfg(feature = "cirrus-desktop-ui")]
    let mut desktop_ui = desktop_ui.into_instance(cirrus_desktop_ui::DesktopUi::new(
        daemon_config.clone(),
        config.clone(),
        jobqueues_ref.clone(),
    )?);

    // run everything
    let instance_name = hostname::get()?.to_string_lossy().into_owned();
    info!("instance name: {}", instance_name);
    info!("running forever...");

    tokio::spawn(async move { jobqueues.run().await.unwrap() });
    tokio::spawn(async move { scheduler.run().await.unwrap() });
    tokio::spawn(async move { configreloader.run().await.unwrap() });
    #[cfg(feature = "cirrus-desktop-ui")]
    tokio::spawn(async move { desktop_ui.run().await.unwrap() });

    tokio::signal::ctrl_c().await?;

    Ok(())
}
