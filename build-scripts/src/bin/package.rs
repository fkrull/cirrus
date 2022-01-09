use std::path::Path;
use tempfile::TempDir;
use xshell::*;

/// Build binaries and a package.
#[derive(argh::FromArgs)]
struct Args {
    /// cirrus version
    #[argh(option)]
    version: String,
    /// cirrus build string
    #[argh(option)]
    build_string: String,
    /// rust target triple
    #[argh(option)]
    target: String,
    /// cargo features for cirrus
    #[argh(option, default = "String::new()")]
    features: String,
    /// RUSTFLAGS to set for the build
    #[argh(option)]
    rustflags: Option<String>,
    /// download and include the restic binary in the package
    #[argh(switch)]
    download_restic: bool,
}

fn main() -> eyre::Result<()> {
    let args: Args = argh::from_env();
    let target = args.target;
    let tmp = TempDir::new()?;

    // compile cirrus
    {
        let _e1 = args.rustflags.as_ref().map(|s| pushenv("RUSTFLAGS", s));
        let _e2 = pushenv("CIRRUS_VERSION", &args.version);
        let _e2 = pushenv("CIRRUS_BUILD_STRING", &args.build_string);
        let _e3 = pushenv("CIRRUS_TARGET", &target);

        let features = args.features;
        cmd!("cargo build --release --target={target} --features={features}").run()?;
        copy_binary(
            Path::new(&format!("target/{}/release/cirrus", target)),
            &tmp.path().join("cirrus"),
        )?;
    }

    // get restic
    if args.download_restic {
        let restic_target = restic_bin::TargetConfig::from_triple(&target)?;
        restic_bin::download(
            &restic_target,
            tmp.path().join(restic_bin::restic_filename(&restic_target)),
        )?;
    }

    // build package
    mkdir_p("public")?;
    let pkg_filename = format!("cirrus_{}.tar.xz", target);
    let pkg_path = Path::new("public").join(&pkg_filename);
    package_tar_xz(tmp.path(), &pkg_path)?;

    Ok(())
}

fn copy_binary(base_src: &Path, base_dst: &Path) -> xshell::Result<()> {
    cp(base_src, base_dst).or_else(|_| {
        cp(
            base_src.with_extension("exe"),
            base_dst.with_extension("exe"),
        )
    })
}

fn package_tar_xz(dir: &Path, dest: &Path) -> eyre::Result<()> {
    let mut xz = xz2::write::XzEncoder::new(std::fs::File::create(dest)?, 6);
    {
        let mut tar = tar::Builder::new(&mut xz);
        for entry in read_dir(dir)? {
            let filename = entry.file_name().ok_or_else(|| eyre::eyre!("not a file"))?;
            tar.append_path_with_name(&entry, filename)?;
        }
        tar.finish()?;
    }
    xz.finish()?;
    Ok(())
}
