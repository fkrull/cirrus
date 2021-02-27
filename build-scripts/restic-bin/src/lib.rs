#[derive(Debug, thiserror::Error)]
#[error("invalid target triple '{0}': {1}")]
pub struct TargetParseError(String, target_lexicon::ParseError);

#[derive(Debug, thiserror::Error)]
pub enum TargetEnvError {
    #[error("error getting '{0}' env var")]
    VarError(String, #[source] std::env::VarError),
    #[error("error parsing target triple")]
    InvalidTargetTriple(#[from] TargetParseError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetConfig {
    triple: target_lexicon::Triple,
}

impl std::fmt::Display for TargetConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.triple)
    }
}

impl TargetConfig {
    pub fn from_triple(triple: impl AsRef<str>) -> Result<TargetConfig, TargetParseError> {
        Self::_from_triple(triple.as_ref())
    }

    fn _from_triple(triple: &str) -> Result<TargetConfig, TargetParseError> {
        use std::str::FromStr;
        let triple = target_lexicon::Triple::from_str(triple)
            .map_err(|e| TargetParseError(triple.to_string(), e))?;
        Ok(TargetConfig { triple })
    }

    pub fn from_env() -> Result<TargetConfig, TargetEnvError> {
        const VAR: &str = "TARGET";
        let target =
            std::env::var(VAR).map_err(|e| TargetEnvError::VarError(VAR.to_string(), e))?;
        Ok(TargetConfig::from_triple(&target)?)
    }
}

pub fn restic_filename(target: &TargetConfig) -> &'static str {
    use target_lexicon::OperatingSystem;
    match target.triple.operating_system {
        OperatingSystem::Windows => "restic.exe",
        _ => "restic",
    }
}

#[cfg(feature = "download")]
mod download_restic;
#[cfg(feature = "download")]
pub use download_restic::*;
