use bundled_exe::{BundledExe, Error};

static BUNDLED_RESTIC: &[u8] = include_bytes!(env!("RESTIC_BIN"));

pub fn bundled_restic() -> Result<BundledExe, Error> {
    BundledExe::new(BUNDLED_RESTIC, env!("RESTIC_FILENAME"))
}
