use std::path::Path;
use tempfile::TempDir;
use xshell::*;
use zip::{write::FileOptions, ZipWriter};

/// Build binaries and a package.
#[derive(argh::FromArgs)]
struct Args {
    /// version
    #[argh(option)]
    version: String,
    /// rust target triple
    #[argh(option)]
    target: String,
    /// cargo features for cirrus
    #[argh(option, default = "String::new()")]
    features: String,
    /// RUSTFLAGS to set for the build
    #[argh(option)]
    rustflags: Option<String>,
}

fn main() -> eyre::Result<()> {
    let args: Args = argh::from_env();

    let target = args.target;
    let bin_ext = bin_ext(&target)?;

    let tmp = TempDir::new()?;

    // compile cirrus
    {
        let _e1 = args.rustflags.as_ref().map(|s| pushenv("RUSTFLAGS", s));
        let _e2 = pushenv("CIRRUS_VERSION", &args.version);

        let features = args.features;
        cmd!("cargo build --release --target={target} --features={features}").run()?;
        cp(
            format!("target/{}/release/cirrus{}", target, bin_ext),
            tmp.path().join(format!("cirrus{}", bin_ext)),
        )?;

        cmd!("cargo build --package=cirrus-daemon-watchdog --release --target={target}").run()?;
        cp(
            format!(
                "target/{}/release/cirrus-daemon-watchdog{}",
                target, bin_ext
            ),
            tmp.path()
                .join(format!("cirrus-daemon-watchdog{}", bin_ext)),
        )?;
    }

    // get restic
    let restic_target = restic_bin::TargetConfig::from_triple(&target)?;
    restic_bin::download(
        &restic_target,
        tmp.path().join(restic_bin::restic_filename(&restic_target)),
    )?;

    // build package
    mkdir_p("public")?;
    let pkg_filename = format!("cirrus_{}.zip", target);
    let pkg_path = Path::new("public").join(&pkg_filename);
    package_zip(tmp.path(), &pkg_path)?;

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

fn package_zip(dir: &Path, dest: &Path) -> eyre::Result<()> {
    use std::{fs::File, io::copy};

    let mut zip = ZipWriter::new(File::create(dest)?);

    for entry in read_dir(dir)? {
        let mut f = File::open(&entry)?;
        zip.start_file(
            entry
                .file_name()
                .and_then(|f| f.to_str())
                .ok_or_else(|| eyre::eyre!("non-UTF8 file name"))?,
            FileOptions::default().unix_permissions(0o755),
        )?;
        copy(&mut f, &mut zip)?;
    }

    zip.finish()?;
    Ok(())
}
