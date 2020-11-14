use sha2::Digest;
use std::{
    fs::File,
    io::{copy, Seek, SeekFrom},
    path::PathBuf,
};

#[derive(Debug)]
pub struct Download {
    url: String,
    to: PathBuf,
    expected_sha256: Option<String>,
    unzip_single: bool,
}

impl Download {
    pub fn new(url: impl Into<String>, to: impl Into<PathBuf>) -> Self {
        Self {
            url: url.into(),
            to: to.into(),
            expected_sha256: None,
            unzip_single: false,
        }
    }

    pub fn expected_sha256(self, expected_sha256: impl Into<String>) -> Self {
        Self {
            expected_sha256: Some(expected_sha256.into()),
            ..self
        }
    }

    pub fn unzip_single(self) -> Self {
        Self {
            unzip_single: true,
            ..self
        }
    }

    pub fn download(self) -> eyre::Result<()> {
        let response = get(&self.url)?;
        let mut tmp = tempfile::tempfile()?;
        copy(&mut response.into_reader(), &mut tmp)?;

        reset(&mut tmp)?;
        if let Some(expected_sha256) = self.expected_sha256 {
            verify_sha256(&mut tmp, &expected_sha256)?;
        }

        let mut out = File::create(self.to)?;
        reset(&mut tmp)?;
        if self.unzip_single {
            let mut arc = zip::ZipArchive::new(&mut tmp)?;
            let mut entry = arc.by_index(0)?;
            copy(&mut entry, &mut out)?;
        } else {
            copy(&mut tmp, &mut out)?;
        }

        Ok(())
    }
}

fn get(url: &str) -> eyre::Result<ureq::Response> {
    let response = ureq::get(url).call();
    if response.error() {
        eyre::bail!("HTTP request failed ({})", response.status_line());
    }
    Ok(response)
}
fn verify_sha256(file: &mut File, sha256: &str) -> eyre::Result<()> {
    let mut digest = sha2::Sha256::new();
    copy(file, &mut digest)?;

    let actual = digest.finalize();
    if actual.as_slice() != hex::decode(sha256)?.as_slice() {
        eyre::bail!(
            "checksum of downloaded file didn't match; expected={}, actual={}",
            sha256,
            hex::encode(actual.as_slice())
        );
    }

    Ok(())
}

fn reset(file: &mut File) -> eyre::Result<()> {
    file.seek(SeekFrom::Start(0))?;
    Ok(())
}
