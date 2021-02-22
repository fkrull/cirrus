use bundled_exe::{BundledExe, Error};
use std::path::Path;

static BUNDLED_RESTIC: &[u8] = include_bytes!(env!("RESTIC_BIN"));

pub fn bundled_restic(dir: impl AsRef<Path>) -> Result<BundledExe, Error> {
    BundledExe::new(BUNDLED_RESTIC, env!("RESTIC_FILENAME"), dir)
}
