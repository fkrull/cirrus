use eyre::{eyre, WrapErr};
use std::path::Path;

pub fn open_config_file(config_path: Option<&Path>) -> eyre::Result<()> {
    let config_path = config_path.ok_or_else(|| eyre!("can't open the configuration file because the configuration was not loaded from a file")
    )?;
    opener::open(config_path)
        .wrap_err_with(|| format!("failed to open config file {}", config_path.display()))
}

pub async fn open_appconfig_file(appconfig_file: &Path) -> eyre::Result<()> {
    tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(appconfig_file)
        .await?;

    opener::open(appconfig_file).wrap_err_with(|| {
        format!(
            "failed to open application config file {}",
            appconfig_file.display()
        )
    })
}
