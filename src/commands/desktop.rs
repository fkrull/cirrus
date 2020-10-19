use eyre::WrapErr;
use std::path::Path;

pub fn open_config_file(config_file: &Path) -> eyre::Result<()> {
    opener::open(config_file)
        .wrap_err_with(|| format!("failed to open config file at {}", config_file.display()))
}
