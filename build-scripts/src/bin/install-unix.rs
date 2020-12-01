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
    let prefix = args.prefix;

    rm_rf("target/install-unix")?;
    mkdir_p(format!("{}/bin", prefix))?;

    // compile cirrus
    let features = args.features;
    cmd!("cargo build --release --features={features} --target={target}").run()?;
    cp(
        format!("target/{}/release/cirrus", target),
        format!("{}/bin/cirrus", prefix),
    )?;

    // install icons
    mkdir_p(format!("{}/share/icons/hicolor", prefix))?;
    cmd!("cp -r build-scripts/linux/icons/hicolor {prefix}/share/icons/").run()?;

    // install desktop file
    mkdir_p(format!("{}/share/applications", prefix))?;
    cp(
        format!("build-scripts/linux/{}.desktop", APP_ID),
        format!("{}/share/applications/{}.desktop", prefix, APP_ID),
    )?;

    Ok(())
}
