use sha2::Digest;
use std::{
    fs::File,
    io::{copy, Seek, SeekFrom},
    path::PathBuf,
};

#[derive(Debug)]
pub enum DecompressMode {
    None,
    UnzipSingle,
    Bunzip2,
}

#[derive(Debug)]
pub struct Download {
    url: String,
    to: PathBuf,
    expected_sha256: Option<String>,
    decompress_mode: DecompressMode,
}

pub fn download(url: impl Into<String>, to: impl Into<PathBuf>) -> Download {
    Download {
        url: url.into(),
        to: to.into(),
        expected_sha256: None,
        decompress_mode: DecompressMode::None,
    }
}

impl Download {
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

    pub fn run(self) -> eyre::Result<()> {
        let response = ureq::get(&self.url).call()?;
        let mut tmp = tempfile::tempfile()?;
        copy(&mut response.into_reader(), &mut tmp)?;

        reset(&mut tmp)?;
        if let Some(expected_sha256) = self.expected_sha256 {
            verify_sha256(&mut tmp, &expected_sha256)?;
        }

        let mut out = File::create(self.to)?;
        reset(&mut tmp)?;
        match self.decompress_mode {
            DecompressMode::None => {
                copy(&mut tmp, &mut out)?;
            }
            DecompressMode::UnzipSingle => {
                let mut arc = zip::ZipArchive::new(&mut tmp)?;
                let mut entry = arc.by_index(0)?;
                copy(&mut entry, &mut out)?;
            }
            DecompressMode::Bunzip2 => {
                let mut bz2_reader = bzip2::read::BzDecoder::new(tmp);
                copy(&mut bz2_reader, &mut out)?;
            }
        }

        Ok(())
    }
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
