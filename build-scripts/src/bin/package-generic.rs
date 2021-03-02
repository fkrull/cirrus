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
            "tar.bz2" => Ok(Package::TarBz2),
            _ => eyre::bail!("invalid package type"),
        }
    }
}

/// Build a generic package.
#[derive(argh::FromArgs)]
struct Args {
    /// directory to package
    #[argh(positional)]
    dir: String,

    /// rust target triple
    #[argh(option)]
    target: String,
    /// package name
    #[argh(option)]
    name: String,
    /// package version
    #[argh(option)]
    version: String,
    /// package type ('zip' or 'tar.bz2')
    #[argh(option)]
    pkg_type: Package,
}

fn main() -> eyre::Result<()> {
    let args: Args = argh::from_env();

    mkdir_p("public")?;
    let pkg_name = format!("{}_{}_{}", args.name, args.version, args.target);
    println!("Building package {}, type {:?}", pkg_name, args.pkg_type);

    match &args.pkg_type {
        Package::Zip => package_zip(&args.dir, &format!("public/{}.zip", pkg_name))?,
        Package::TarBz2 => package_tar_bz2(&args.dir, &format!("public/{}.tar.bz2", pkg_name))?,
    }

    Ok(())
}

fn package_zip(dir: &str, dest: &str) -> eyre::Result<()> {
    use std::{fs::File, io::copy};
    use zip::{write::FileOptions, write::ZipWriter};

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

fn package_tar_bz2(dir: &str, dest: &str) -> eyre::Result<()> {
    use std::fs::File;

    let mut tar = tar::Builder::new(bzip2::write::BzEncoder::new(
        File::create(dest)?,
        bzip2::Compression::best(),
    ));

    for entry in read_dir(dir)? {
        let f = File::open(&entry)?;
        let mut header = tar::Header::new_gnu();
        header.set_size(f.metadata()?.len());
        header.set_mode(0o755);
        header.set_cksum();
        tar.append_data(&mut header, entry.file_name().unwrap(), f)?;
    }

    tar.finish()?;
    Ok(())
}
