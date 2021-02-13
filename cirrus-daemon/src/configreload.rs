use cirrus_actor::{Actor, Messages};
use cirrus_core::model::Config;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct ConfigReload {
    pub new_config: Arc<Config>,
}

#[derive(Debug)]
pub struct ConfigReloader {
    config: Arc<Config>,
    configreload_sink: Messages<ConfigReload>,
}

impl ConfigReloader {
    pub fn new(config: Arc<Config>, configreload_sink: Messages<ConfigReload>) -> Self {
        ConfigReloader {
            config,
            configreload_sink,
        }
    }
}

#[async_trait::async_trait]
impl Actor for ConfigReloader {
    type Message = ();
    type Error = eyre::Report;

    async fn on_message(&mut self, _message: Self::Message) -> Result<(), Self::Error> {
        unimplemented!()
    }
}
