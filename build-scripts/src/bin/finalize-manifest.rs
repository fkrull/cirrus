use build_scripts::{Manifest, ManifestItem};
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
    let mut items = Vec::new();
    let json_files = read_dir(&args.dir)?
        .into_iter()
        .filter(|o| o.extension() == Some(OsStr::new("json")));
    for json_file in json_files {
        let item = ManifestItem::deserialize_json(&std::fs::read_to_string(&json_file)?)?;
        let item = ManifestItem {
            url: args.url_pattern.replace("__FILENAME__", &item.filename),
            ..item
        };
        items.push(item);
        std::fs::remove_file(json_file)?;
    }

    // write manifest file
    let manifest = Manifest { items };
    std::fs::write(
        Path::new(&args.dir).join("manifest.json"),
        manifest.serialize_json(),
    )?;

    Ok(())
}
