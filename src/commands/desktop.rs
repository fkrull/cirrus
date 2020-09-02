use anyhow::Context;
use std::path::Path;

pub fn open_config_file(config_file: &Path) -> anyhow::Result<()> {
    opener::open(config_file).context(format!(
        "failed to open config file at {}",
        config_file.display()
    ))
}
