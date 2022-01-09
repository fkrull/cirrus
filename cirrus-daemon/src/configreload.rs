use cirrus_actor::{Actor, ActorRef, Messages};
use cirrus_core::model::Config;
use notify::Watcher;
use std::{cell::RefCell, sync::Arc};
use tracing::{error, info, warn};

#[derive(Debug, Clone)]
pub struct ConfigReload {
    pub new_config: Arc<Config>,
}

pub struct ConfigReloader {
    config: Arc<Config>,
    configreload_sink: Messages<ConfigReload>,
    watcher: notify::RecommendedWatcher,
}

impl ConfigReloader {
    pub fn new(
        config: Arc<Config>,
        self_ref: ActorRef<Message>,
        configreload_sink: Messages<ConfigReload>,
    ) -> eyre::Result<Self> {
        let self_ref = RefCell::new(self_ref);
        let watcher = notify::recommended_watcher(move |ev| {
            if let Err(err) = self_ref.borrow_mut().send(Message(ev)) {
                error!("error sending config reload trigger message: {:?}", err);
            }
        })?;

        Ok(ConfigReloader {
            config,
            configreload_sink,
            watcher,
        })
    }

    async fn reload_config(&mut self) -> eyre::Result<()> {
        // TODO: debounce events
        if let Some(config_path) = &self.config.source {
            let result = Config::from_file(config_path).await;
            match result {
                Ok(config) => {
                    info!("reloaded configuration from {}", config_path.display());
                    let config = Arc::new(config);
                    self.config = config;
                    self.configreload_sink.send(ConfigReload {
                        new_config: self.config.clone(),
                    })?;
                }
                Err(err) => {
                    warn!("failed to reload configuration: {:?}", err);
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct Message(notify::Result<notify::Event>);

#[async_trait::async_trait]
impl Actor for ConfigReloader {
    type Message = Message;
    type Error = eyre::Report;

    async fn on_message(&mut self, message: Self::Message) -> Result<(), Self::Error> {
        let ev = message.0?;
        if !ev.kind.is_create() && !ev.kind.is_modify() {
            // don't care about this one
            return Ok(());
        }
        self.reload_config().await
    }

    async fn on_start(&mut self) -> Result<(), Self::Error> {
        use notify::RecursiveMode::NonRecursive;

        if let Some(config_path) = &self.config.source {
            info!(
                "watching configuration file {} for changes",
                config_path.display()
            );
            self.watcher.watch(config_path, NonRecursive)?;
        }
        Ok(())
    }

    async fn on_close(&mut self) -> Result<(), Self::Error> {
        if let Some(config_path) = &self.config.source {
            self.watcher.unwatch(config_path)?;
        }
        Ok(())
    }
}
