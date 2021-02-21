#[derive(Debug, thiserror::Error)]
#[error("failed to read target config from build.rs environment")]
pub struct TargetEnvError(#[from] std::env::VarError);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetConfig {
    pub os: String,
    pub arch: String,
    pub endian: String,
}

impl TargetConfig {
    pub fn from_env() -> Result<TargetConfig, TargetEnvError> {
        use std::env::var;

        let os = var("CARGO_CFG_TARGET_OS")?;
        let arch = var("CARGO_CFG_TARGET_ARCH")?;
        let endian = var("CARGO_CFG_TARGET_ENDIAN")?;
        Ok(TargetConfig { os, arch, endian })
    }
}

pub fn restic_filename(target: &TargetConfig) -> &'static str {
    match target.os.as_str() {
        "windows" => "restic.exe",
        _ => "restic",
    }
}

#[cfg(feature = "download")]
mod download_restic;
#[cfg(feature = "download")]
pub use download_restic::*;
