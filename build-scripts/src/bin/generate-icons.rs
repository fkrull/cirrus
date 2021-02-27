use std::path::Path;
use xshell::*;

fn main() -> eyre::Result<()> {
    status_icons()?;
    Ok(())
}

fn status_icons() -> eyre::Result<()> {
    let icons = [
        ("cirrus-idle.light", vec!["light"]),
        ("cirrus-idle.dark", vec!["dark"]),
        ("cirrus-running.light", vec!["light", "running"]),
        ("cirrus-running.dark", vec!["dark", "running"]),
    ];
    let sizes = [16, 24, 32, 48];

    for (name, objects) in &icons {
        let mut pngs = Vec::new();
        for &size in &sizes {
            let png = format!("cirrus-desktop-ui/src/resources/{}/{}.png", size, name);
            export_merged_png("icons/symbolic-icon.svg", &png, size, &objects)?;
            pngs.push(png);
        }
        cmd!("convert {pngs...} cirrus-desktop-ui/src/resources/{name}.ico").run()?;
    }

    Ok(())
}

fn export_merged_png(
    svg: impl AsRef<Path>,
    png: impl AsRef<Path>,
    size: u32,
    objects: &[&str],
) -> eyre::Result<()> {
    let svg = svg.as_ref();
    let png = png.as_ref();
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
