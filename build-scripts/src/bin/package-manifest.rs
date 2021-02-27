use std::{collections::HashMap, fs::File, path::Path};
use xshell::*;

/// Create a package manifest.
#[derive(argh::FromArgs)]
struct Args {
    /// package file directory
    #[argh(option)]
    package_dir: String,
    /// file download URL prefix
    #[argh(option)]
    url_prefix: String,
}

#[derive(serde::Serialize)]
struct ManifestEntry {
    url: String,
    sha256: String,
}

#[derive(serde::Serialize)]
#[serde(transparent)]
struct Manifest(HashMap<String, ManifestEntry>);

fn main() -> eyre::Result<()> {
    let args: Args = argh::from_env();

    // create manifest
    let manifest = read_dir(&args.package_dir)?
        .iter()
        .map(|p| manifest_entry(p, &args.url_prefix))
        .collect::<Result<HashMap<_, _>, _>>()?;

    // write manifest file
    let manifest_path = Path::new(&args.package_dir).join("manifest.json");
    let mut manifest_file = File::create(&manifest_path)?;
    serde_json::to_writer_pretty(&mut manifest_file, &Manifest(manifest))?;

    // write manifest checksum
    let manifest_sha256 = sha256(&manifest_path)?;
    std::fs::write(manifest_path.with_extension("json.sha256"), manifest_sha256)?;

    Ok(())
}

fn manifest_entry(path: &Path, url_prefix: &str) -> eyre::Result<(String, ManifestEntry)> {
    let filename = path
        .file_name()
        .ok_or_else(|| eyre::eyre!("invalid path {}", path.display()))?
        .to_str()
        .ok_or_else(|| eyre::eyre!("non-UTF8 path {}", path.display()))?;
    let url = format!("{}{}", url_prefix, filename);
    let sha256 = sha256(path)?;

    Ok((filename.to_string(), ManifestEntry { url, sha256 }))
}

fn sha256(path: &Path) -> eyre::Result<String> {
    use sha2::Digest;
    use std::io::copy;

    let mut digest = sha2::Sha256::new();
    let mut f = File::open(path)?;
    copy(&mut f, &mut digest)?;
    let sha256 = hex::encode(digest.finalize().as_slice());
    Ok(sha256)
}
