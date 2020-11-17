use xshell::*;

mod download;

pub use download::*;
use std::path::Path;

pub fn restic(target: &str, dest_file: &str) -> eyre::Result<()> {
    match target {
        "x86_64-pc-windows-msvc" => {
            download("https://github.com/restic/restic/releases/download/v0.11.0/restic_0.11.0_windows_amd64.zip", dest_file)
                .expected_sha256("4d9ec99ceec71df88f47c5ebae5fdd15474f7d36e9685a655830c2fc89ad9153")
                .unzip_single()
                .run()
        }
        "x86_64-unknown-linux-musl" => {
            download("https://github.com/restic/restic/releases/download/v0.11.0/restic_0.11.0_linux_amd64.bz2", dest_file)
                .expected_sha256("f559e774c91f1201ffddba74d5758dec8342ad2b50a3bcd735ccb0c88839045c")
                .bunzip2()
                .run()
        }
        "armv7-unknown-linux-musleabihf" => {
            download("https://github.com/restic/restic/releases/download/v0.11.0/restic_0.11.0_linux_arm.bz2", dest_file)
                .expected_sha256("bcefbd70874b8198be4635b5c64b15359a7c28287d274e02d5177c4933ad3f71")
                .bunzip2()
                .run()
        }
        _ => eyre::bail!("unknown target {}", target),
    }
}

pub fn export_merged_png(
    svg: impl AsRef<Path>,
    png: impl AsRef<Path>,
    size: u32,
    objects: &[&str],
) -> eyre::Result<()> {
    _export_merged_png(svg.as_ref(), png.as_ref(), size, objects)
}

fn _export_merged_png(svg: &Path, png: &Path, size: u32, objects: &[&str]) -> eyre::Result<()> {
    let tmp = tempfile::tempdir()?;

    mkdir_p(png.parent().unwrap())?;
    let mut object_filenames = Vec::new();
    let size = size.to_string();
    for &object in objects {
        let object_filename = tmp.path().join(format!("{}.png", object));
        cmd!(
            "inkscape {svg}
                --export-type=png
                --export-filename={object_filename}
                --export-width={size} --export-height={size}
                --export-area-page
                --export-id={object} --export-id-only"
        )
        .run()?;
        object_filenames.push(object_filename);
    }

    cmd!(
        "convert
            -background transparent
            {object_filenames...}
            -layers flatten
            -define png:color-type=6
            {png}"
    )
    .run()?;

    Ok(())
}
