use std::path::Path;
use xshell::*;

/// Generate icons from the raw SVG.
#[derive(argh::FromArgs)]
#[argh(subcommand, name = "generate-icons")]
pub struct Args {}

pub fn main(_args: Args) -> eyre::Result<()> {
    let sh = Shell::new()?;
    status_icons(&sh)?;
    Ok(())
}

fn status_icons(sh: &Shell) -> eyre::Result<()> {
    let icons = [
        ("cirrus-idle.light", vec!["light"]),
        ("cirrus-idle.dark", vec!["dark"]),
        ("cirrus-running.light", vec!["light", "running"]),
        ("cirrus-running.dark", vec!["dark", "running"]),
        ("cirrus-suspend.light", vec!["light", "suspend"]),
        ("cirrus-suspend.dark", vec!["dark", "suspend"]),
    ];
    let sizes = [16, 24, 32, 48];

    for (name, objects) in &icons {
        let mut pngs = Vec::new();
        for &size in &sizes {
            let png = format!("cirrus-desktop-ui/src/resources/{}/{}.png", size, name);
            export_merged_png(sh, "icons/symbolic-icon.svg", &png, size, objects)?;
            pngs.push(png);
        }
        cmd!(
            sh,
            "convert {pngs...} cirrus-desktop-ui/src/resources/{name}.ico"
        )
        .run()?;
    }

    Ok(())
}

fn export_merged_png(
    sh: &Shell,
    svg: impl AsRef<Path>,
    png: impl AsRef<Path>,
    size: u32,
    objects: &[&str],
) -> eyre::Result<()> {
    let svg = svg.as_ref();
    let png = png.as_ref();
    let tmp = tempfile::tempdir()?;

    sh.create_dir(png.parent().unwrap())?;
    let mut object_filenames = Vec::new();
    let size = size.to_string();
    for &object in objects {
        let object_filename = tmp.path().join(format!("{}.png", object));
        cmd!(
            sh,
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
        sh,
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
