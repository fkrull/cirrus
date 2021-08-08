pub mod model;
pub mod restic;
pub mod secrets;

pub const VERSION: Option<&'static str> = match option_env!("CIRRUS_VERSION") {
    Some(v) if !v.is_empty() => Some(v),
    _ => None,
};

pub const TARGET: Option<&'static str> = match option_env!("CIRRUS_TARGET") {
    Some(v) if !v.is_empty() => Some(v),
    _ => None,
};
