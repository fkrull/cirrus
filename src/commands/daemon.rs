use crate::cli;
use cirrus_core::{cache::Cache, config::Config, restic::Restic, secrets::Secrets};
use cirrus_daemon::*;
use std::{path::PathBuf, sync::Arc};
use tokio::process::Command;

async fn run_daemon(
    restic: Restic,
    secrets: Secrets,
    cache: Cache,
    config: Config,
) -> eyre::Result<()> {
    let restic = Arc::new(restic);
    let secrets = Arc::new(secrets);
    let config = Arc::new(config);
    let mut events = events::Builder::new_with_capacity(128);

    let mut suspend_service = suspend::SuspendService::new(&mut events);
    let mut job_queues = job::queues::JobQueues::new(
        &mut events,
        restic.clone(),
        secrets.clone(),
        cache.clone(),
        *suspend_service.get_suspend(),
    );
    let mut scheduler = scheduler::Scheduler::new(config.clone(), &mut events);
    let mut config_reload_service =
        config_reload::ConfigReloadService::new(config.clone(), &mut events)?;
    let mut shutdown_service = shutdown::ShutdownService::new(&mut events);
    let mut signal_handler = signal_handler::SignalHandler::new(&mut events);
    let status_icon = cirrus_desktop_ui::StatusIcon::new(
        config.clone(),
        &mut events,
        *suspend_service.get_suspend(),
    );

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
    tokio::spawn(async move {
        if let Err(error) = status_icon.run().await {
            tracing::warn!(%error, "error while running the status icon");
        }
    });

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
    // TODO: maybe change into a separate top-level thing?
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

pub async fn main(
    args: cli::daemon::Cli,
    restic: Restic,
    secrets: Secrets,
    config: Config,
    cache: Cache,
) -> eyre::Result<()> {
    if args.supervisor {
        run_supervisor().await
    } else {
        run_daemon(restic, secrets, cache, config).await
    }
}
