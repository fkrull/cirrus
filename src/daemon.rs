use cirrus_actor::Messages;
use cirrus_core::{model::Config, restic::Restic, restic_util, secrets::Secrets};
use cirrus_daemon::*;
use std::{path::PathBuf, sync::Arc};
use tracing::{info, warn};

async fn data_dir() -> eyre::Result<PathBuf> {
    use dirs_next as dirs;

    let data_dir = dirs::data_dir()
        .ok_or_else(|| eyre::eyre!("failed to get data dir path"))?
        .join("cirrus");
    tokio::fs::create_dir_all(&data_dir).await?;
    Ok(data_dir)
}

async fn setup_daemon_logger() -> eyre::Result<()> {
    use tracing::Level;
    use tracing_subscriber::{
        filter::LevelFilter,
        fmt::{format::FmtSpan, layer, time::ChronoLocal},
        layer::SubscriberExt,
        util::SubscriberInitExt,
        Registry,
    };

    const TIME_FORMAT: &str = "%Y-%m-%d %H:%M:%S%Z";

    let stdout_layer = layer()
        .with_ansi(true)
        .with_target(false)
        .with_timer(ChronoLocal::with_format(String::from(TIME_FORMAT)));

    let data_dir = data_dir().await?;
    let file_layer = layer()
        .with_ansi(false)
        .with_span_events(FmtSpan::CLOSE)
        .with_timer(ChronoLocal::with_format(String::from(TIME_FORMAT)))
        .with_writer(move || tracing_appender::rolling::never(&data_dir, "cirrus.log"));

    Registry::default()
        .with(LevelFilter::from(Level::INFO))
        .with(stdout_layer)
        .with(file_layer)
        .try_init()?;

    Ok(())
}

pub async fn run(restic: Restic, secrets: Secrets, config: Config) -> eyre::Result<()> {
    setup_daemon_logger().await?;

    let restic_version = restic_util::restic_version(&restic)
        .await
        .unwrap_or_else(|e| {
            warn!("failed to query restic version: {}", e);
            "<unknown restic version>".to_string()
        });

    let restic = Arc::new(restic);
    let secrets = Arc::new(secrets);
    let config = Arc::new(config);
    #[allow(unused_variables)]
    let daemon_config = Arc::new(daemon_config::DaemonConfig {
        versions: daemon_config::Versions { restic_version },
        ..Default::default()
    });

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
        jobqueues_ref.clone().into(),
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
