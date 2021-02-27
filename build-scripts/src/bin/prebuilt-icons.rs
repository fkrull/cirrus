use build_scripts::*;
use xshell::*;

fn main() -> eyre::Result<()> {
    status_icons()?;
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
