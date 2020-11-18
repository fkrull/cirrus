use build_scripts::export_merged_png;

fn main() -> eyre::Result<()> {
    // APPX 44x44 logo
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

    // APPX 150x150 tile
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
