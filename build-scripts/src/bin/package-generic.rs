use restic_bin::restic_filename;
use xshell::*;

/// Build a container image.
#[derive(argh::FromArgs)]
struct Args {
    /// rust target triple
    #[argh(option)]
    target: String,
    /// cargo features for cirrus
    #[argh(option, default = "String::new()")]
    features: String,
    /// include cirrus-gui binary
    #[argh(switch)]
    cirrus_gui: bool,
    /// use rust-lld instead of the system linker
    #[argh(switch)]
    rust_lld: bool,
}

#[cfg(windows)]
const BIN_EXT: &str = ".exe";

#[cfg(not(windows))]
const BIN_EXT: &str = "";

fn main() -> eyre::Result<()> {
    let args: Args = argh::from_env();
    let target = args.target;

    // create package dir
    let package_dir = format!("target/package-{}", target);
    mkdir_p(&package_dir)?;

    // compile cirrus
    {
        let _e = if args.rust_lld {
            pushenv("RUSTFLAGS", "-Clinker=rust-lld")
        } else {
            pushenv("RUSTFLAGS", "")
        };

        let features = args.features;
        cmd!("cargo build --release --target={target} --features={features}").run()?;
        cp(
            format!("target/{}/release/cirrus{}", target, BIN_EXT),
            format!("{}/cirrus{}", package_dir, BIN_EXT),
        )?;

        if args.cirrus_gui {
            cmd!("cargo build --package=cirrus-gui --release --target={target}").run()?;
            cp(
                format!("target/{}/release/cirrus-gui{}", target, BIN_EXT),
                format!("{}/cirrus-gui{}", package_dir, BIN_EXT),
            )?;
        }
    }

    // get restic
    let target = restic_bin::TargetConfig::from_triple(target)?;
    restic_bin::download(
        &target,
        format!("{}/{}", package_dir, restic_filename(&target)),
    )?;

    Ok(())
}
