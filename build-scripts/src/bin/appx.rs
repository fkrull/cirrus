use xshell::*;

/// Build an appx package.
#[derive(argh::FromArgs)]
struct Args {
    /// rust compile target
    #[argh(option)]
    target: String,
    /// package version
    #[argh(option)]
    version: String,
    /// certificate thumbprint
    #[argh(option)]
    cert_thumbprint: String,
    /// use system certificate store
    #[argh(switch)]
    use_system_cert_store: bool,
}

fn main() -> eyre::Result<()> {
    let args: Args = argh::from_env();
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
    build_scripts::restic(&target, "target/appx/restic.exe")?;

    // copy files
    for path in read_dir("build-scripts/windows/appx")? {
        cp(&path, "target/appx/")?;
    }

    // expand manifest
    let appx_arch = appx_arch(&target)?;
    let manifest = read_file("target/appx/AppxManifest.xml")?
        .replace("$APPX_VERSION", &args.version)
        .replace("$APPX_ARCH", appx_arch);
    write_file("target/appx/AppxManifest.xml", manifest)?;

    // build appx
    let appx_filename = format!("Cirrus_{}_{}.appx", args.version, appx_arch);
    cmd!("makeappx pack /h SHA256 /o /d target/appx /p target/{appx_filename}").run()?;

    // sign
    let cert_thumbprint = args.cert_thumbprint;
    let cert_store_flags = if args.use_system_cert_store {
        Some("/sm")
    } else {
        None
    };
    cmd!("SignTool sign /fd SHA256 /a /sha1 {cert_thumbprint} {cert_store_flags...} target/{appx_filename}")
        .run()?;

    // generate appinstaller file
    let appinstaller = format!("Cirrus_{}.appinstaller", appx_arch);
    let appinstaller_xml = read_file("build-scripts/windows/Cirrus.appinstaller")?
        .replace("$APPX_VERSION", &args.version)
        .replace("$APPX_ARCH", appx_arch)
        .replace("$APPX_FILENAME", &appx_filename)
        .replace("$APPINSTALLER", &appinstaller);
    write_file(format!("target/{}", appinstaller), appinstaller_xml)?;

    Ok(())
}

fn appx_arch(target: &str) -> eyre::Result<&str> {
    Ok(match target {
        "x86_64-pc-windows-msvc" => "x64",
        "i686-pc-windows-msvc" => "x86",
        _ => eyre::bail!("unknown target {}", target),
    })
}
