use std::{ffi::OsStr, path::Path};
use tempfile::NamedTempFile;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("i/o error")]
    TempfileIoError(#[from] std::io::Error),
}

#[derive(Debug)]
pub struct BundledExe(NamedTempFile);

impl BundledExe {
    pub fn new(bytes: impl AsRef<[u8]>, filename: impl AsRef<OsStr>) -> Result<BundledExe, Error> {
        Self::_new(bytes.as_ref(), filename.as_ref())
    }

    fn _new(mut bytes: &[u8], filename: &OsStr) -> Result<BundledExe, Error> {
        let mut tmp = tempfile::Builder::new().suffix(filename).tempfile()?;
        std::io::copy(&mut bytes, &mut tmp)?;
        Ok(BundledExe(tmp))
    }

    pub fn path(&self) -> &Path {
        self.0.path()
    }
}