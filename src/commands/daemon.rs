use crate::cli;
use cirrus_core::{config::Config, restic::Restic, secrets::Secrets};
use cirrus_daemon::*;
use shindig::Events;
use std::{path::PathBuf, sync::Arc};
use tokio::process::Command;

async fn setup_daemon_logger(log_file: Option<&PathBuf>) -> eyre::Result<()> {
    use tracing::Level;
    use tracing_subscriber::{
        filter::LevelFilter,
        fmt::{format::FmtSpan, layer, time::LocalTime},
        layer::SubscriberExt,
        util::SubscriberInitExt,
        Registry,
    };

    let builder = Registry::default()
        .with(LevelFilter::from(Level::INFO))
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
    setup_daemon_logger(args.log_file.as_ref()).await?;

    let restic = Arc::new(restic);
    let secrets = Arc::new(secrets);
    let config = Arc::new(config);
    let events = Events::new_with_capacity(64);

    #[cfg(feature = "cirrus-desktop-ui")]
    let desktop_ui = match cirrus_desktop_ui::DesktopUi::new(config.clone(), events.clone()) {
        Ok(desktop_ui) => Some(desktop_ui),
        Err(err) => {
            tracing::warn!("failed to start desktop UI: {}", err);
            None
        }
    };

    let mut jobqueues = job_queues::JobQueues::new(events.clone(), restic.clone(), secrets.clone());
    let mut scheduler = scheduler::Scheduler::new(config.clone(), events.clone());
    let mut configreloader = configreload::ConfigReloader::new(config.clone(), events.clone())?;

    // run everything
    let instance_name = hostname::get()?.to_string_lossy().into_owned();
    tracing::info!("instance name: {}", instance_name);
    if let Some(version) = cirrus_core::VERSION {
        tracing::info!("cirrus: {}", version);
    }
    match restic.version_string().await {
        Ok(restic_version) => tracing::info!("restic: {}", restic_version),
        Err(e) => tracing::warn!("failed to query restic version: {}", e),
    }
    tracing::info!("running forever...");

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
