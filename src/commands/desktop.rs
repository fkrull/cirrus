use eyre::WrapErr;
use std::path::Path;

pub fn open_config_file(config_file: &Path) -> eyre::Result<()> {
    opener::open(config_file)
        .wrap_err_with(|| format!("failed to open config file {}", config_file.display()))
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
