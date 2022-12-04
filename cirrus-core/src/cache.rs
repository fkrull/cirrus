use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Cache(PathBuf);

impl Cache {
    const CACHEDIR_TAG_FILENAME: &'static str = "CACHEDIR.TAG";

    const CACHEDIR_TAG_CONTENT: &'static str = "Signature: 8a477f597d28d172789f06886806bc55
# This file is a cache directory tag created by cirrus.
# For information about cache directory tags see https://bford.info/cachedir/
";

    pub fn new(path: PathBuf) -> Cache {
        Cache(path)
    }

    pub async fn get(&self) -> eyre::Result<&Path> {
        tokio::fs::create_dir_all(&self.0).await?;
        tokio::fs::write(
            self.0.join(Self::CACHEDIR_TAG_FILENAME),
            Self::CACHEDIR_TAG_CONTENT,
        )
        .await?;
        Ok(&self.0)
    }
}
