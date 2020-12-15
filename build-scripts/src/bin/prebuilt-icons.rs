use build_scripts::*;
use xshell::*;

fn main() -> eyre::Result<()> {
    status_icons()?;
    appx_44x44()?;
    appx_150x150()?;
    xdg_icons()?;

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

fn appx_44x44() -> eyre::Result<()> {
    for &size in &[16, 24, 32, 48, 256] {
        // plated
        let png = format!(
            "build-scripts/windows/appx/Square44x44Logo.targetsize-{}.png",
            size
        );
        export_merged_png("icons/symbolic-icon.svg", png, size, &["light"])?;

        // unplated dark
        let png = format!(
            "build-scripts/windows/appx/Square44x44Logo.targetsize-{}_altform-unplated.png",
            size
        );
        export_merged_png("icons/symbolic-icon.svg", png, size, &["light"])?;

        // unplated light
        let png = format!(
            "build-scripts/windows/appx/Square44x44Logo.targetsize-{}_altform-lightunplated.png",
            size
        );
        export_merged_png("icons/symbolic-icon.svg", png, size, &["dark"])?;
    }

    Ok(())
}

fn appx_150x150() -> eyre::Result<()> {
    for &scale in &[100, 200, 300, 400] {
        let px = 150 * (scale / 100);
        let png = format!(
            "build-scripts/windows/appx/Square150x150Logo.scale-{}.png",
            scale
        );
        export_merged_png("icons/windows-tile-square.svg", png, px, &["icon"])?;
    }

    Ok(())
}

fn xdg_icons() -> eyre::Result<()> {
    let app_id = "io.gitlab.fkrull.cirrus.Cirrus";

    for &size in &[16, 24, 32, 48, 64, 128, 256] {
        let png = format!(
            "build-scripts/unix/icons/hicolor/{}x{}/apps/{}.png",
            size, size, app_id
        );
        export_merged_png("icons/app-icon.svg", png, size, &["icon"])?;
    }

    Ok(())
}
