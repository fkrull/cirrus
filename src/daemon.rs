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
    let (jobqueues_actor, jobqueues_ref) = cirrus_actor::new_actor();
    let (scheduler_actor, scheduler_ref) = cirrus_actor::new_actor();
    let (configreload_actor, _) = cirrus_actor::new_actor();
    #[cfg(feature = "cirrus-desktop-ui")]
    let (desktop_ui_actor, desktop_ui_ref) = cirrus_actor::new_actor();

    // connect multicast
    let jobstatus_messages = Messages::default();
    #[cfg(feature = "cirrus-desktop-ui")]
    let jobstatus_messages = jobstatus_messages.also_to(desktop_ui_ref.clone());

    let configreload_messages = Messages::default().also_to(scheduler_ref);
    #[cfg(feature = "cirrus-desktop-ui")]
    let configreload_messages = configreload_messages.also_to(desktop_ui_ref);

    // create actor instances
    let mut jobqueues_actor = jobqueues_actor.into_instance(job_queues::JobQueues::new(
        jobstatus_messages,
        restic.clone(),
        secrets.clone(),
    ));

    let mut scheduler_actor = scheduler_actor.into_instance(scheduler::Scheduler::new(
        config.clone(),
        jobqueues_ref.clone(),
    ));

    let mut configreload_actor = configreload_actor.into_instance(
        configreload::ConfigReloader::new(config.clone(), configreload_messages),
    );

    #[cfg(feature = "cirrus-desktop-ui")]
    let mut desktop_ui_actor = desktop_ui_actor.into_instance(cirrus_desktop_ui::DesktopUi::new(
        daemon_config.clone(),
        config.clone(),
        jobqueues_ref.clone(),
    )?);

    // run actor instances
    tokio::spawn(async move { jobqueues_actor.run().await.unwrap() });
    tokio::spawn(async move { scheduler_actor.run().await.unwrap() });
    tokio::spawn(async move { configreload_actor.run().await.unwrap() });
    #[cfg(feature = "cirrus-desktop-ui")]
    tokio::spawn(async move { desktop_ui_actor.run().await.unwrap() });

    let instance_name = hostname::get()?.to_string_lossy().into_owned();
    info!("instance name: {}", instance_name);

    info!("running forever...");
    tokio::signal::ctrl_c().await?;

    Ok(())
}
