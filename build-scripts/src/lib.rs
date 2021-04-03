use nanoserde::{DeJson, SerJson};

#[derive(Debug, DeJson, SerJson)]
pub struct ManifestPackage {
    pub name: String,
    pub os: String,
    pub arch: String,
    pub build: String,
    pub version: String,
    pub url: String,
    pub sha256: String,
}

#[derive(Debug, DeJson, SerJson)]
pub struct Manifest {
    pub packages: Vec<ManifestPackage>,
}
