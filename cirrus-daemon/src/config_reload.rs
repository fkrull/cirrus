use crate::{shutdown::ShutdownAcknowledged, shutdown::ShutdownRequested};
use cirrus_core::config::Config;
use notify::Watcher;
use std::sync::Arc;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ConfigReload {
    pub new_config: Arc<Config>,
}

#[derive(Debug, Clone)]
struct NotifyEvent(notify::Event);

events::subscriptions! {
    ShutdownRequested,
    NotifyEvent,
}

pub struct ConfigReloadService {
    events: Subscriptions,
    config: Arc<Config>,
    watcher: notify::RecommendedWatcher,
}

impl ConfigReloadService {
    pub fn new(config: Arc<Config>, events: &mut events::Builder) -> eyre::Result<Self> {
        let notify_sender = events.typed_sender::<NotifyEvent>();
        let watcher = notify::recommended_watcher(move |ev| match ev {
            Ok(event) => {
                notify_sender.send(NotifyEvent(event));
            }
            Err(error) => tracing::error!(?error, "notify error"),
        })?;

        Ok(ConfigReloadService {
            events: Subscriptions::subscribe(events),
            config,
            watcher,
        })
    }

    async fn reload_config(&mut self) -> eyre::Result<()> {
        // TODO: debounce events
        if let Some(config_path) = &self.config.source {
            let result = Config::parse_file(config_path).await;
            match result {
                Ok(config) => {
                    tracing::info!(path = %config_path.display(), "reloaded configuration");
                    let config = Arc::new(config);
                    self.config = config;
                    self.events.send(ConfigReload {
                        new_config: self.config.clone(),
                    });
                }
                Err(error) => {
                    tracing::warn!(?error, "failed to reload configuration");
                }
            }
        }
        Ok(())
    }

    #[tracing::instrument(name = "ConfigReloadService", skip_all)]
    pub async fn run(&mut self) -> eyre::Result<()> {
        self.start_watch()?;
        loop {
            tokio::select! {
                notify_event = self.events.NotifyEvent.recv() => self.handle_notify_event(notify_event?).await?,
                shutdown_requested = self.events.ShutdownRequested.recv() => {
                    self.handle_shutdown(shutdown_requested?).await?;
                    break Ok(());
                },
            }
        }
    }

    fn start_watch(&mut self) -> eyre::Result<()> {
        use notify::RecursiveMode::NonRecursive;

        if let Some(config_path) = &self.config.source {
            tracing::info!(
                path = %config_path.display(),
                "watching configuration file for changes"
            );
            self.watcher.watch(config_path, NonRecursive)?;
        }
        Ok(())
    }

    async fn handle_notify_event(&mut self, ev: NotifyEvent) -> eyre::Result<()> {
        if !ev.0.kind.is_create() && !ev.0.kind.is_modify() {
            // don't care about this one
            return Ok(());
        }
        self.reload_config().await
    }

    async fn handle_shutdown(&mut self, _: ShutdownRequested) -> eyre::Result<()> {
        tracing::debug!("received shutdown event");
        if let Some(config_path) = &self.config.source {
            self.watcher.unwatch(config_path)?;
        }
        self.events.send(ShutdownAcknowledged);
        Ok(())
    }
}
