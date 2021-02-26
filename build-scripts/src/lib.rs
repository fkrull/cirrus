use std::{path::Path, str::FromStr};
use target_lexicon::{OperatingSystem, Triple};
use xshell::*;

pub fn bin_ext(target: &str) -> eyre::Result<&'static str> {
    let bin_ext = match Triple::from_str(target)
        .map_err(|e| eyre::eyre!("{}", e))?
        .operating_system
    {
        OperatingSystem::Windows => ".exe",
        _ => "",
    };
    Ok(bin_ext)
}

pub fn restic(target: &str, dest_file: &str) -> eyre::Result<()> {
    let target = restic_bin::TargetConfig::from_triple(target)?;
    restic_bin::download(&target, dest_file)?;
    Ok(())
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
