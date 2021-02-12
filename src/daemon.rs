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

    let (jobqueues_actor, jobqueues_ref) = cirrus_actor::new_actor();
    #[cfg(feature = "cirrus-desktop-ui")]
    let (desktop_ui_actor, desktop_ui_ref) = cirrus_actor::new_actor();

    let jobstatus_messages = Messages::new_discarding();
    #[cfg(feature = "cirrus-desktop-ui")]
    let jobstatus_messages = jobstatus_messages.also_to(desktop_ui_ref);

    let mut jobqueues_actor = jobqueues_actor.into_instance(job_queues::JobQueues::new(
        jobstatus_messages,
        restic.clone(),
        secrets.clone(),
    ));

    #[cfg(feature = "cirrus-desktop-ui")]
    let mut desktop_ui_actor = desktop_ui_actor.into_instance(cirrus_desktop_ui::DesktopUi::new(
        daemon_config.clone(),
        config.clone(),
        jobqueues_ref.clone(),
    )?);

    let mut scheduler = scheduler::Scheduler::new(config.clone(), jobqueues_ref.clone());

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
