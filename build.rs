use restic_bin::*;
use std::path::{Path, PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let target = TargetConfig::from_env()?;
    let out = Path::new(&std::env::var("OUT_DIR")?).join(restic_filename(&target));
    download(&target, out)?;
    Ok(())
}
