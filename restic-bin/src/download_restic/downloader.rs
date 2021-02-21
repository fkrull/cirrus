use sha2::Digest;
use std::{
    fs::File,
    io::{copy, Seek, SeekFrom},
    path::PathBuf,
};

#[derive(Debug, thiserror::Error)]
#[error("HTTP error")]
pub struct HttpError(#[from] ureq::Error);

#[derive(Debug, thiserror::Error)]
#[error("zip error")]
pub struct ZipError(#[from] zip::result::ZipError);

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid checksum string {0}")]
    InvalidHexString(String, #[source] hex::FromHexError),
    #[error("checksum of downloaded file didn't match; expected={expected}, actual={actual}")]
    InvalidChecksum { expected: String, actual: String },
    #[error("HTTP request failed")]
    HttpRequestFailed(#[from] HttpError),
    #[error("download failed")]
    DownloadFailed(#[source] std::io::Error),
    #[error("invalid zip file")]
    InvalidZipFile(#[from] ZipError),
    #[error("zip decompression failed")]
    UnzipFailed(#[source] std::io::Error),
    #[error("bz2 decompression failed")]
    Bunzip2Failed(#[source] std::io::Error),
    #[error("generic i/o error")]
    IoError(#[from] std::io::Error),
}

#[derive(Debug)]
pub enum DecompressMode {
    None,
    UnzipSingle,
    Bunzip2,
}

#[derive(Debug)]
pub struct Downloader {
    url: String,
    to: PathBuf,
    expected_sha256: Option<String>,
    decompress_mode: DecompressMode,
}

pub fn downloader(url: impl Into<String>, to: impl Into<PathBuf>) -> Downloader {
    Downloader {
        url: url.into(),
        to: to.into(),
        expected_sha256: None,
        decompress_mode: DecompressMode::None,
    }
}

impl Downloader {
    pub fn expected_sha256(mut self, expected_sha256: impl Into<String>) -> Self {
        self.expected_sha256 = Some(expected_sha256.into());
        self
    }

    pub fn decompress_mode(mut self, mode: DecompressMode) -> Self {
        self.decompress_mode = mode;
        self
    }

    pub fn unzip_single(self) -> Self {
        self.decompress_mode(DecompressMode::UnzipSingle)
    }

    pub fn bunzip2(self) -> Self {
        self.decompress_mode(DecompressMode::Bunzip2)
    }

    pub fn run(self) -> Result<(), Error> {
        let response = ureq::get(&self.url).call().map_err(HttpError)?;
        let mut tmp = tempfile::tempfile()?;
        copy(&mut response.into_reader(), &mut tmp).map_err(Error::DownloadFailed)?;

        reset(&mut tmp)?;
        if let Some(expected_sha256) = &self.expected_sha256 {
            verify_sha256(&mut tmp, expected_sha256)?;
        }

        let mut out = File::create(&self.to)?;
        reset(&mut tmp)?;
        self.decompress(&mut tmp, &mut out)?;

        Ok(())
    }

    fn decompress(&self, mut tmp: &mut File, mut out: &mut File) -> Result<(), Error> {
        match self.decompress_mode {
            DecompressMode::None => {
                copy(&mut tmp, &mut out)?;
            }
            DecompressMode::UnzipSingle => {
                let mut arc = zip::ZipArchive::new(&mut tmp).map_err(ZipError)?;
                let mut entry = arc.by_index(0).map_err(ZipError)?;
                copy(&mut entry, &mut out).map_err(|e| Error::UnzipFailed(e))?;
            }
            DecompressMode::Bunzip2 => {
                let mut bz2_reader = bzip2::read::BzDecoder::new(tmp);
                copy(&mut bz2_reader, &mut out).map_err(|e| Error::Bunzip2Failed(e))?;
            }
        }
        Ok(())
    }
}

fn verify_sha256(file: &mut File, sha256: &str) -> Result<(), Error> {
    let mut digest = sha2::Sha256::new();
    copy(file, &mut digest)?;

    let actual = digest.finalize();
    let expected =
        hex::decode(sha256).map_err(|e| Error::InvalidHexString(sha256.to_owned(), e))?;
    if actual.as_slice() != expected.as_slice() {
        return Err(Error::InvalidChecksum {
            expected: sha256.to_owned(),
            actual: hex::encode(actual.as_slice()),
        });
    }

    Ok(())
}

fn reset(file: &mut File) -> std::io::Result<()> {
    file.seek(SeekFrom::Start(0))?;
    Ok(())
}
