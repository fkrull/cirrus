use package::download;
use std::path::Path;
use xshell::*;

/// Build an appx package.
#[derive(argh::FromArgs)]
struct Args {
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
    let args: Args = argh::from_env();
    let target = args.target.as_str();
    let certificate = args.certificate;
    let certificate_password = std::env::var("CERTIFICATE_PASSWORD")?;

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
    download(args.restic_url, "target/appx/restic.exe")
        .expected_sha256(args.restic_sha256)
        .unzip_single()
        .run()?;

    // copy files
    for path in read_dir("package/appx")? {
        cp(
            &path,
            Path::new("target/appx/DEST").with_file_name(path.file_name().unwrap()),
        )?;
    }

    // expand manifest
    let appx_arch = match args.target.as_str() {
        "x86_64-pc-windows-msvc" => "x64",
        "i686-pc-windows-msvc" => "x86",
        _ => eyre::bail!("unknown architecture"),
    };
    let manifest = read_file("target/appx/AppxManifest.xml")?
        .replace("$APPX_VERSION", &args.version)
        .replace("$APPX_ARCH", appx_arch);
    write_file("target/appx/AppxManifest.xml", manifest)?;

    // build appx
    cmd!("makeappx pack /h SHA256 /o /d target/appx /p target/Cirrus.appx").run()?;

    // sign
    cmd!(
        "SignTool sign /fd SHA256 /sm /a /f {certificate} /p {certificate_password} target/Cirrus.appx"
    )
    .dont_echo_cmd()
    .run()?;

    Ok(())
}
