use package::Download;
use std::path::Path;
use xshell::*;

#[derive(argh::FromArgs)]
/// Reach new heights.
struct BuildAppx {
    /// rust compile target
    #[argh(option)]
    target: String,
    /// restic package url
    #[argh(option)]
    restic_url: String,
    /// restic expected SHA256
    #[argh(option)]
    restic_sha256: String,
    /// package version
    #[argh(option)]
    version: String,
    /// certificate file
    #[argh(option)]
    certificate: String,
}

fn main() -> eyre::Result<()> {
    let args: BuildAppx = argh::from_env();
    let target = args.target.as_str();

    // compile cirrus
    cmd!("cargo build --release --features=desktop --target={target}").run()?;
    cmd!("cargo build --release --package=cirrus-windows-wrapper --target={target}").run()?;

    rm_rf("target/appx")?;
    mkdir_p("target/appx")?;
    cp(
        format!("target/{}/release/cirrus.exe", args.target),
        "target/appx/cirrus.exe",
    )?;
    cp(
        format!("target/{}/release/cirrus-windows-wrapper.exe", args.target),
        "target/appx/cirrus-windows-wrapper.exe",
    )?;

    // download restic
    Download::new(args.restic_url, "target/appx/restic.exe")
        .expected_sha256(args.restic_sha256)
        .unzip_single()
        .download()?;

    // create manifest
    let appx_arch = match args.target.as_str() {
        "x86_64-pc-windows-msvc" => "x64",
        "i686-pc-windows-msvc" => "x86",
        _ => eyre::bail!("unknown architecture"),
    };
    let manifest = read_file("package/windows/AppxManifest.xml")?
        .replace("$APPX_VERSION", &args.version)
        .replace("$APPX_ARCH", appx_arch);
    write_file("target/appx/AppxManifest.xml", manifest)?;

    // copy images
    for png in glob::glob("package/windows/*.png")? {
        let png = png?;
        cp(
            &png,
            Path::new("target/appx/DEST").with_file_name(png.file_name().unwrap()),
        )?;
    }

    // build appx
    cmd!("makeappx pack /h SHA256 /o /d target/appx /p target/Cirrus.appx").run()?;

    Ok(())
}
