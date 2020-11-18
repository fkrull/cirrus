use build_scripts::export_merged_png;

const SIZES: &[u32] = &[16, 24, 32, 48, 256];

fn main() -> eyre::Result<()> {
    export_merged_png(
        "icons/symbolic-icon.svg",
        "build-scripts/windows/appx/Square44x44Logo.png",
        44,
        &["light"],
    )?;

    for &size in SIZES {
        let png = format!(
            "build-scripts/windows/appx/Square44x44Logo.targetsize-{}_altform-unplated.png",
            size
        );
        export_merged_png("icons/symbolic-icon.svg", png, size, &["light"])?;

        let png = format!(
            "build-scripts/windows/appx/Square44x44Logo.targetsize-{}_altform-lightunplated.png",
            size
        );
        export_merged_png("icons/symbolic-icon.svg", png, size, &["dark"])?;
    }

    Ok(())
}
