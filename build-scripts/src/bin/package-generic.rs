use restic_bin::restic_filename;
use std::str::FromStr;
use xshell::*;

#[derive(Debug)]
enum Package {
    Zip,
    TarBz2,
}

impl FromStr for Package {
    type Err = eyre::Report;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "zip" => Ok(Package::Zip),
            "tbz" | "tarbz2" => Ok(Package::TarBz2),
            _ => eyre::bail!("invalid package type"),
        }
    }
}

/// Build a container image.
#[derive(argh::FromArgs)]
struct Args {
    /// rust target triple
    #[argh(option)]
    target: String,
    /// package version specifier
    #[argh(option)]
    version: String,
    /// cargo features for cirrus
    #[argh(option, default = "String::new()")]
    features: String,
    /// include cirrus-gui binary
    #[argh(switch)]
    cirrus_gui: bool,
    /// linker to use
    #[argh(option)]
    linker: Option<String>,
    /// package type to create
    #[argh(option)]
    package: Option<Package>,
}

fn main() -> eyre::Result<()> {
    let args: Args = argh::from_env();
    let target = args.target;
    let bin_ext = build_scripts::bin_ext(&target)?;

    // create package dir
    let name = format!("cirrus_{}_{}", args.version, target);
    let package_dir = format!("target/{}", name);
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

    // build package
    if let Some(package) = args.package {
        mkdir_p("public")?;
        println!("Building package with type {:?}", package);
        match package {
            Package::Zip => package_zip(&package_dir, &format!("public/{}.zip", name))?,
            Package::TarBz2 => package_tar_bz2(&package_dir, &format!("public/{}.tar.bz2", name))?,
        }
    }

    Ok(())
}

fn package_zip(dir: &str, dest: &str) -> eyre::Result<()> {
    use std::{
        fs::{read_dir, File},
        io::copy,
    };
    use zip::{write::FileOptions, write::ZipWriter};

    let mut zip = ZipWriter::new(File::create(dest)?);

    for entry in read_dir(dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }

        let mut f = File::open(entry.path())?;
        zip.start_file(
            entry
                .file_name()
                .to_str()
                .ok_or_else(|| eyre::eyre!("non-UTF8 file name"))?,
            FileOptions::default().unix_permissions(0o755),
        )?;
        copy(&mut f, &mut zip)?;
    }

    zip.finish()?;
    Ok(())
}

fn package_tar_bz2(dir: &str, dest: &str) -> eyre::Result<()> {
    use std::fs::{read_dir, File};

    let mut tar = tar::Builder::new(bzip2::write::BzEncoder::new(
        File::create(dest)?,
        bzip2::Compression::best(),
    ));

    for entry in read_dir(dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }

        let f = File::open(entry.path())?;
        let mut header = tar::Header::new_gnu();
        header.set_size(f.metadata()?.len());
        header.set_mode(0o755);
        header.set_cksum();
        tar.append_data(&mut header, entry.file_name(), f)?;
    }

    tar.finish()?;
    Ok(())
}
