use build_scripts::*;
use xshell::*;

/// Install binaries, icons, and desktop metadata files for Unix-likes.
#[derive(argh::FromArgs)]
struct Args {
    /// rust compile target
    #[argh(option)]
    target: String,
    /// cargo build features
    #[argh(option, default = r#"String::from("desktop")"#)]
    features: String,
    /// install prefix
    #[argh(option, default = r#"String::from("/usr/local")"#)]
    prefix: String,
}

const APP_ID: &str = "io.gitlab.fkrull.cirrus.Cirrus";

fn main() -> eyre::Result<()> {
    let args: Args = argh::from_env();
    let target = args.target;

    rm_rf("target/install-unix")?;
    mkdir_p(format!("{}/bin", args.prefix))?;

    // compile cirrus
    let features = args.features;
    cmd!("cargo build --release --features={features} --target={target}").run()?;
    cp(
        format!("target/{}/release/cirrus", target),
        format!("{}/bin/cirrus", args.prefix),
    )?;

    // generate icons
    for &size in &[16, 24, 32, 48, 64, 128, 256] {
        let png = format!(
            "{}/share/icons/hicolor/{}x{}/apps/{}.png",
            args.prefix, size, size, APP_ID
        );
        export_merged_png("icons/app-icon.svg", png, size, &["icon"])?;
    }

    Ok(())
}
