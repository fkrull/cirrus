use cirrus_core::model::Config;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct ConfigReloaded {
    pub new_config: Arc<Config>,
}
