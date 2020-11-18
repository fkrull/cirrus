use build_scripts::export_merged_png;
use xshell::*;

struct IconConfig {
    name: &'static str,
    objects: Vec<&'static str>,
}

const SIZES: &[u32] = &[16, 24, 32, 48];

fn main() -> eyre::Result<()> {
    let icon_configs = [
        IconConfig {
            name: "cirrus-idle.light",
            objects: vec!["light"],
        },
        IconConfig {
            name: "cirrus-idle.dark",
            objects: vec!["dark"],
        },
        IconConfig {
            name: "cirrus-running.light",
            objects: vec!["light", "running"],
        },
        IconConfig {
            name: "cirrus-running.dark",
            objects: vec!["dark", "running"],
        },
    ];

    for icon in &icon_configs {
        let mut pngs = Vec::new();
        for &size in SIZES {
            let png = format!("cirrus-desktop-ui/src/resources/{}/{}.png", size, icon.name);
            export_merged_png("icons/symbolic-icon.svg", &png, size, &icon.objects)?;
            pngs.push(png);
        }
        let name = icon.name;
        cmd!("convert {pngs...} cirrus-desktop-ui/src/resources/{name}.ico").run()?;
    }

    Ok(())
}
