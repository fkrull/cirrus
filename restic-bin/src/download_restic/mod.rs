use crate::TargetConfig;
use std::path::Path;

mod downloader;
mod urls;

#[derive(Debug, thiserror::Error)]
pub enum DownloadError {
    #[error("no download available for {0:?}")]
    NoDownloadForTarget(TargetConfig),
    #[error("download failed")]
    DownloadFailed(#[from] downloader::Error),
}

pub fn download(target: &TargetConfig, dest: impl AsRef<Path>) -> Result<(), DownloadError> {
    _download(target, dest.as_ref())
}

fn _download(target: &TargetConfig, dest: &Path) -> Result<(), DownloadError> {
    let urls = urls::Urls::default();
    let url_and_checksum = urls
        .url_and_checksum(target)
        .ok_or_else(|| DownloadError::NoDownloadForTarget(target.clone()))?;
    downloader::downloader(&url_and_checksum.url, dest)
        .decompress_mode(url_and_checksum.decompress_mode())
        .expected_sha256(url_and_checksum.checksum)
        .run()?;
    Ok(())
}
