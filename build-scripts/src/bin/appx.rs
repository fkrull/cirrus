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
    /// directly install the package
    #[argh(switch)]
    register: bool,
}

fn main() -> eyre::Result<()> {
    let args: Args = argh::from_env();
    let target = args.target.as_str();

    // compile cirrus
    cmd!("cargo run --package=build-scripts --bin=package-generic -- --target {target} --features desktop --cirrus-gui").run()?;

    // copy files
    rm_rf("target/appx")?;
    mkdir_p("target/appx")?;

    for path in read_dir("target/package")? {
        cp(&path, "target/appx/")?;
    }

    for path in read_dir("build-scripts/windows/appx")? {
        cp(&path, "target/appx/")?;
    }

    // expand manifest
    let appx_arch = appx_arch(&target)?;
    let manifest = read_file("target/appx/AppxManifest.xml")?
        .replace("$APPX_VERSION", &args.version)
        .replace("$APPX_ARCH", appx_arch);
    write_file("target/appx/AppxManifest.xml", manifest)?;

    // create package resource index
    // see https://docs.microsoft.com/en-us/windows/msix/desktop/desktop-to-uwp-manual-conversion#optional-add-target-based-unplated-assets
    {
        let _d = pushd("target/appx")?;
        cmd!("makepri createconfig /cf priconfig.xml /dq en-US").run()?;
        cmd!("makepri new /pr . /cf priconfig.xml").run()?;
    }

    // build package or register it directly
    if args.register {
        let ps_script = "Add-AppxPackage -Register target/appx/AppxManifest.xml";
        cmd!("powershell -Command {ps_script}").run()?;
    } else {
        build_package(&args, appx_arch)?;
    }

    Ok(())
}

fn appx_arch(target: &str) -> eyre::Result<&str> {
    Ok(match target {
        "x86_64-pc-windows-msvc" => "x64",
        "i686-pc-windows-msvc" => "x86",
        _ => eyre::bail!("unknown target {}", target),
    })
}

fn build_package(args: &Args, appx_arch: &str) -> eyre::Result<()> {
    // build appx
    mkdir_p("public")?;
    let appx_filename = format!("Cirrus_{}_{}.appx", args.version, appx_arch);
    cmd!("makeappx pack /h SHA256 /o /d target/appx /p public/{appx_filename}").run()?;

    // sign
    let cert_thumbprint = &args.cert_thumbprint;
    let cert_store_flags = if args.use_system_cert_store {
        Some("/sm")
    } else {
        None
    };
    cmd!("SignTool sign /fd SHA256 /a /sha1 {cert_thumbprint} {cert_store_flags...} public/{appx_filename}")
        .run()?;

    // generate appinstaller file
    let appinstaller = format!("Cirrus_{}.appinstaller", appx_arch);
    let appinstaller_xml = read_file("build-scripts/windows/Cirrus.appinstaller")?
        .replace("$APPX_VERSION", &args.version)
        .replace("$APPX_ARCH", appx_arch)
        .replace("$APPX_FILENAME", &appx_filename)
        .replace("$APPINSTALLER", &appinstaller);
    write_file(format!("public/{}", appinstaller), appinstaller_xml)?;

    Ok(())
}
