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
    /// linker to use
    #[argh(option)]
    linker: Option<String>,
}

fn main() -> eyre::Result<()> {
    let args: Args = argh::from_env();
    let target = args.target;
    let bin_ext = build_scripts::bin_ext(&target)?;

    // create package dir
    let package_dir = format!("target/package-{}", target);
    mkdir_p(&package_dir)?;

    // compile cirrus
    {
        let mut rustflags = std::env::var_os("RUSTFLAGS").unwrap_or_default();
        if let Some(linker) = &args.linker {
            rustflags.push(" -Clinker=");
            rustflags.push(linker);
        }
        let _e = pushenv("RUSTFLAGS", rustflags);

        let features = args.features;
        cmd!("cargo build --release --target={target} --features={features}").run()?;
        cp(
            format!("target/{}/release/cirrus{}", target, bin_ext),
            format!("{}/cirrus{}", package_dir, bin_ext),
        )?;

        if args.cirrus_gui {
            cmd!("cargo build --package=cirrus-gui --release --target={target}").run()?;
            cp(
                format!("target/{}/release/cirrus-gui{}", target, bin_ext),
                format!("{}/cirrus-gui{}", package_dir, bin_ext),
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
