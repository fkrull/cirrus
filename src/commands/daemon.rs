use crate::cli;
use cirrus_actor::Messages;
use cirrus_core::{config::Config, restic::Restic, secrets::Secrets};
use cirrus_daemon::*;
use std::{path::PathBuf, sync::Arc};
use tokio::process::Command;
use tracing::{info, warn};

async fn cache_dir() -> eyre::Result<PathBuf> {
    use dirs_next as dirs;

    let data_dir = dirs::cache_dir()
        .ok_or_else(|| eyre::eyre!("failed to get cache dir path"))?
        .join("cirrus");
    tokio::fs::create_dir_all(&data_dir).await?;
    Ok(data_dir)
}

async fn setup_daemon_logger() -> eyre::Result<()> {
    use tracing::Level;
    use tracing_subscriber::{
        filter::LevelFilter,
        fmt::{format::FmtSpan, layer, time::LocalTime},
        layer::SubscriberExt,
        util::SubscriberInitExt,
        Registry,
    };

    let time_format = time::macros::format_description!(
        "[year]-[month]-[day] [hour repr:24]:[minute]:[second][offset_hour sign:mandatory]:[offset_minute]"
    );

    let stdout_layer = layer().with_ansi(true).with_target(false).without_time();

    let log_file_dir = cache_dir().await?;
    let file_layer = layer()
        .with_ansi(false)
        .with_span_events(FmtSpan::CLOSE)
        .with_timer(LocalTime::new(time_format))
        .with_writer(move || tracing_appender::rolling::never(&log_file_dir, "cirrus.log"));

    Registry::default()
        .with(LevelFilter::from(Level::INFO))
        .with(stdout_layer)
        .with(file_layer)
        .try_init()?;

    Ok(())
}

async fn run_daemon(
    _args: cli::daemon::Cli,
    restic: Restic,
    secrets: Secrets,
    config: Config,
) -> eyre::Result<()> {
    setup_daemon_logger().await?;

    let restic = Arc::new(restic);
    let secrets = Arc::new(secrets);
    let config = Arc::new(config);

    // declare actors
    let jobqueues = cirrus_actor::new();
    let scheduler = cirrus_actor::new();
    let configreloader = cirrus_actor::new();
    let mut jobstatus_sink = Messages::default();
    let mut configreload_sink = Messages::default().also_to(scheduler.actor_ref());
    let job_sink = Messages::default().also_to(jobqueues.actor_ref());

    // create actor instances
    #[cfg(feature = "cirrus-desktop-ui")]
    let desktop_ui = {
        let desktop_ui_builder = cirrus_actor::new();
        let desktop_ui_ref = desktop_ui_builder.actor_ref();
        match cirrus_desktop_ui::DesktopUi::new(config.clone(), job_sink.clone()) {
            Ok(desktop_ui) => {
                jobstatus_sink = jobstatus_sink.also_to(desktop_ui_ref.clone());
                configreload_sink = configreload_sink.also_to(desktop_ui_ref);
                Some(desktop_ui_builder.into_instance(desktop_ui))
            }
            Err(err) => {
                warn!("failed to start desktop UI: {}", err);
                None
            }
        }
    };

    let mut jobqueues = jobqueues.into_instance(job_queues::JobQueues::new(
        jobstatus_sink,
        restic.clone(),
        secrets.clone(),
    ));

    let mut scheduler =
        scheduler.into_instance(scheduler::Scheduler::new(config.clone(), job_sink.clone()));

    let configreloader_ref = configreloader.actor_ref();
    let mut configreloader = configreloader.into_instance(configreload::ConfigReloader::new(
        config.clone(),
        configreloader_ref,
        configreload_sink,
    )?);

    // run everything
    let instance_name = hostname::get()?.to_string_lossy().into_owned();
    info!("instance name: {}", instance_name);
    match restic.version_string().await {
        Ok(restic_version) => info!("restic version: {}", restic_version),
        Err(e) => warn!("failed to query restic version: {}", e),
    }
    info!("running forever...");

    tokio::spawn(async move { jobqueues.run().await.unwrap() });
    tokio::spawn(async move { scheduler.run().await.unwrap() });
    tokio::spawn(async move { configreloader.run().await.unwrap() });
    #[cfg(feature = "cirrus-desktop-ui")]
    if let Some(mut desktop_ui) = desktop_ui {
        tokio::spawn(async move { desktop_ui.run().await.unwrap() });
    }

    tokio::signal::ctrl_c().await?;

    Ok(())
}

async fn run_supervisor() -> eyre::Result<()> {
    let cirrus_exe = std::env::current_exe()?;
    loop {
        let exit_status = Command::new(&cirrus_exe)
            .arg("daemon")
            .spawn()?
            .wait()
            .await;
        match exit_status {
            Ok(s) if s.success() => break,
            _ => continue,
        }
    }
    Ok(())
}

pub async fn run(
    args: cli::daemon::Cli,
    restic: Restic,
    secrets: Secrets,
    config: Config,
) -> eyre::Result<()> {
    if args.supervisor {
        run_supervisor().await
    } else {
        run_daemon(args, restic, secrets, config).await
    }
}
