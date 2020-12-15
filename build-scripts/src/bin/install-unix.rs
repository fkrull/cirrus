use xshell::*;

/// Install binaries, icons, and desktop metadata files for Unix-likes.
#[derive(argh::FromArgs)]
struct Args {
    /// rust compile target
    #[argh(option)]
    target: Option<String>,
    /// install prefix
    #[argh(option, default = r#"String::from("/usr/local")"#)]
    destdir: String,
}

const APP_ID: &str = "io.gitlab.fkrull.cirrus.Cirrus";

fn main() -> eyre::Result<()> {
    let args: Args = argh::from_env();
    let destdir = args.destdir;

    // install cirrus
    mkdir_p(format!("{}/bin", destdir))?;
    if let Some(target) = args.target {
        cp(
            format!("target/{}/release/cirrus", target),
            format!("{}/bin/cirrus", destdir),
        )?;
    } else {
        cp("target/release/cirrus", format!("{}/bin/cirrus", destdir))?;
    }

    // install icons
    mkdir_p(format!("{}/share/icons/hicolor", destdir))?;
    cmd!("cp -r build-scripts/unix/icons/hicolor {destdir}/share/icons/").run()?;

    // install desktop file
    mkdir_p(format!("{}/share/applications", destdir))?;
    cp(
        format!("build-scripts/unix/{}.desktop", APP_ID),
        format!("{}/share/applications/{}.desktop", destdir, APP_ID),
    )?;

    Ok(())
}
