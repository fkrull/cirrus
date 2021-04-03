use build_scripts::{Manifest, ManifestPackage};
use nanoserde::{DeJson, SerJson};
use std::{ffi::OsStr, path::Path};
use xshell::*;

/// Create a package manifest.
#[derive(argh::FromArgs)]
struct Args {
    /// package file directory
    #[argh(positional)]
    dir: String,
    /// file download URL pattern; __FILENAME__ will be replaced with the file name
    #[argh(option)]
    url_pattern: String,
}

fn main() -> eyre::Result<()> {
    let args: Args = argh::from_env();

    // build manifest
    let mut packages = Vec::new();
    let json_files = read_dir(&args.dir)?
        .into_iter()
        .filter(|o| o.extension() == Some(OsStr::new("json")));
    for json_file in json_files {
        let filename = json_file
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| eyre::eyre!("invalid filename"))?;
        let item = ManifestPackage::deserialize_json(&std::fs::read_to_string(&json_file)?)?;
        let item = ManifestPackage {
            url: args.url_pattern.replace("__FILENAME__", filename),
            ..item
        };
        packages.push(item);
        std::fs::remove_file(json_file)?;
    }

    // write manifest file
    let manifest = Manifest { packages };
    std::fs::write(
        Path::new(&args.dir).join("manifest.json"),
        manifest.serialize_json(),
    )?;

    Ok(())
}
