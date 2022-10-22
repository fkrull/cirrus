use crate::cli;
use cirrus_core::{config::Config, restic::Restic, secrets::Secrets};
use cirrus_daemon::*;
use std::{path::PathBuf, sync::Arc};
use tokio::process::Command;

async fn setup_daemon_logger(level: cli::LogLevel, log_file: Option<&PathBuf>) -> eyre::Result<()> {
    use tracing_subscriber::{
        filter::LevelFilter,
        fmt::{format::FmtSpan, layer, time::LocalTime},
        layer::SubscriberExt,
        util::SubscriberInitExt,
        Registry,
    };

    let builder = Registry::default()
        .with(LevelFilter::from_level(level.into()))
        .with(layer().with_ansi(true).with_target(false).without_time());

    if let Some(log_file) = log_file {
        let time_format = time::macros::format_description!(
            "[year]-[month]-[day] [hour repr:24]:[minute]:[second][offset_hour sign:mandatory]:[offset_minute]"
        );

        let file = std::fs::File::options()
            .append(true)
            .create(true)
            .open(log_file)?;
        builder
            .with(
                layer()
                    .with_ansi(false)
                    .with_span_events(FmtSpan::CLOSE)
                    .with_timer(LocalTime::new(time_format))
                    .with_writer(file),
            )
            .try_init()?;
    } else {
        builder.try_init()?;
    }

    Ok(())
}

async fn run_daemon(
    args: cli::daemon::Cli,
    restic: Restic,
    secrets: Secrets,
    config: Config,
) -> eyre::Result<()> {
    setup_daemon_logger(args.log_level, args.log_file.as_ref()).await?;

    let restic = Arc::new(restic);
    let secrets = Arc::new(secrets);
    let config = Arc::new(config);
    let mut events = events::Builder::new_with_capacity(128);

    let mut suspend_service = suspend::SuspendService::new(&mut events);
    let mut job_queues = job::queues::JobQueues::new(
        &mut events,
        restic.clone(),
        secrets.clone(),
        *suspend_service.get_suspend(),
    );
    let mut scheduler = scheduler::Scheduler::new(config.clone(), &mut events);
    let mut config_reload_service =
        config_reload::ConfigReloadService::new(config.clone(), &mut events)?;
    let mut shutdown_service = shutdown::ShutdownService::new(&mut events);
    let mut signal_handler = signal_handler::SignalHandler::new(&mut events);

    #[cfg(feature = "cirrus-desktop-ui")]
    let status_icon = match cirrus_desktop_ui::StatusIcon::new(
        config.clone(),
        &mut events,
        *suspend_service.get_suspend(),
    ) {
        Ok(status_icon) => Some(status_icon),
        Err(error) => {
            tracing::warn!(%error, "failed to create status icon");
            None
        }
    };

    // run everything
    let instance_name = hostname::get()?.to_string_lossy().into_owned();
    tracing::info!(instance_name);
    if let Some(version) = cirrus_core::VERSION {
        tracing::info!(cirrus_version = %version);
    }
    match restic.version_string().await {
        Ok(restic_version) => tracing::info!(%restic_version),
        Err(error) => tracing::warn!(%error, "failed to query restic version"),
    }

    tokio::spawn(async move { job_queues.run().await.unwrap() });
    tokio::spawn(async move { scheduler.run().await.unwrap() });
    tokio::spawn(async move { config_reload_service.run().await.unwrap() });
    tokio::spawn(async move { shutdown_service.run().await.unwrap() });
    tokio::spawn(async move { suspend_service.run().await.unwrap() });
    tokio::spawn(async move { signal_handler.run().await.unwrap() });
    #[cfg(feature = "cirrus-desktop-ui")]
    if let Some(status_icon) = status_icon {
        tokio::spawn(async move { status_icon.run().await.unwrap() });
    }

    tracing::info!("running forever...");
    futures::future::pending::<eyre::Result<()>>().await
}

async fn log_file_dir() -> eyre::Result<PathBuf> {
    use dirs_next as dirs;

    let log_file_dir = dirs::data_dir()
        .ok_or_else(|| eyre::eyre!("can't determine data directory for log file"))?
        .join("cirrus");
    tokio::fs::create_dir_all(&log_file_dir).await?;
    Ok(log_file_dir)
}

async fn run_supervisor() -> eyre::Result<()> {
    let cirrus_exe = std::env::current_exe()?;
    let log_file = log_file_dir().await?.join("cirrus.log");
    loop {
        let exit_status = Command::new(&cirrus_exe)
            .arg("daemon")
            .arg("--log-file")
            .arg(&log_file)
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
