use restic_bin::restic_filename;
use xshell::*;

/// Build all binaries for a package.
#[derive(argh::FromArgs)]
struct Args {
    /// rust target triple
    #[argh(option)]
    target: String,
    /// cargo features for cirrus
    #[argh(option, default = "String::new()")]
    features: String,
    /// include cirrus-restart-helper binary
    #[argh(switch)]
    cirrus_restart_helper: bool,
    /// RUSTFLAGS to set for the build
    #[argh(option)]
    rustflags: Option<String>,
}

fn main() -> eyre::Result<()> {
    let args: Args = argh::from_env();
    let target = args.target;
    let bin_ext = bin_ext(&target)?;

    // create package dir
    let package_dir = "target/binaries";
    rm_rf(package_dir)?;
    mkdir_p(package_dir)?;

    // compile cirrus
    {
        let _e = if let Some(rustflags) = &args.rustflags {
            Some(pushenv("RUSTFLAGS", rustflags))
        } else {
            None
        };

        let features = args.features;
        cmd!("cargo build --release --target={target} --features={features}").run()?;
        cp(
            format!("target/{}/release/cirrus{}", target, bin_ext),
            format!("{}/cirrus{}", package_dir, bin_ext),
        )?;

        if args.cirrus_restart_helper {
            cmd!("cargo build --package=cirrus-restart-helper --release --target={target}")
                .run()?;
            cp(
                format!("target/{}/release/cirrus-restart-helper{}", target, bin_ext),
                format!("{}/cirrus-restart-helper{}", package_dir, bin_ext),
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

fn bin_ext(target: &str) -> eyre::Result<&'static str> {
    use std::str::FromStr;
    use target_lexicon::{OperatingSystem, Triple};

    let bin_ext = match Triple::from_str(target)
        .map_err(|e| eyre::eyre!("{}", e))?
        .operating_system
    {
        OperatingSystem::Windows => ".exe",
        _ => "",
    };
    Ok(bin_ext)
}
