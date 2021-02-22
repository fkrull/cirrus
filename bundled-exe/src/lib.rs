use std::io::Write;
use std::path::PathBuf;
use std::{ffi::OsStr, path::Path};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("i/o error")]
    IoError(#[from] std::io::Error),
    #[error("persist error")]
    CantPersist(#[from] tempfile::PersistError),
}

#[derive(Debug)]
pub struct BundledExe(PathBuf);

impl BundledExe {
    pub fn new(
        bytes: impl AsRef<[u8]>,
        filename: impl AsRef<OsStr>,
        dir: impl AsRef<Path>,
    ) -> Result<BundledExe, Error> {
        Self::_new(bytes.as_ref(), filename.as_ref(), dir.as_ref())
    }

    fn _new(mut bytes: &[u8], filename: &OsStr, dir: &Path) -> Result<BundledExe, Error> {
        let sha256 = sha256(bytes);
        let versioned_dir = dir.join(sha256);
        std::fs::create_dir_all(&versioned_dir)?;
        let mut tmp = tempfile::NamedTempFile::new_in(&versioned_dir)?;
        std::io::copy(&mut bytes, &mut tmp)?;
        let versioned_filename = versioned_dir.join(filename);
        tmp.persist(&versioned_filename)?;
        Ok(BundledExe(versioned_filename))
    }

    pub fn path(&self) -> &Path {
        &self.0
    }
}

fn sha256(bytes: &[u8]) -> String {
    use sha2::Digest;
    let mut digest = sha2::Sha256::new();
    digest.write_all(bytes).unwrap();
    let actual = digest.finalize();
    hex::encode(actual.as_slice())
}
