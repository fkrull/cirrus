pub mod model;
pub mod restic;
pub mod secrets;

pub const VERSION: Option<&'static str> = option_env!("CIRRUS_RELEASE_VERSION");
