use nanoserde::{DeJson, SerJson};

#[derive(Debug, DeJson, SerJson)]
pub struct ManifestItem {
    pub name: String,
    pub version: String,
    pub arch: String,
    pub filename: String,
    pub url: String,
    pub sha256: String,
}

#[derive(Debug, DeJson, SerJson)]
pub struct Manifest {
    pub items: Vec<ManifestItem>,
}
