use build_scripts::TargetVars;
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
    /// build the restic binary from the vendored source and include it in the package
    #[argh(switch)]
    build_restic: bool,
    /// build and statically link libdbus
    #[argh(switch)]
    static_dbus: bool,
}

fn main() -> eyre::Result<()> {
    let sh = Shell::new()?;
    let args: Args = argh::from_env();
    let target = args.target;
    let tmp = TempDir::new()?;
    let target_vars = TargetVars::for_target(&target)?;
    let ext = target_vars.extension;

    // compile restic
    if args.build_restic {
        let _e1 = sh.push_env("GOOS", target_vars.go_os);
        let _e2 = sh.push_env("GOARCH", target_vars.go_arch);
        let _e3 = sh.push_env("GOARM", target_vars.go_arm.unwrap_or(""));

        let bin = format!("restic{ext}");
        let bin_path = format!("../target/{target}/{bin}");
        let _cd = sh.push_dir("restic");
        cmd!(sh, "go build -ldflags '-w -s' -o {bin_path} ./cmd/restic").run()?;
        sh.copy_file(bin_path, tmp.path().join(bin))?;
    }

    // compile dbus
    let dbus_link_args = if args.static_dbus && target_vars.uses_dbus {
        let dbus_build_dir = format!("./target/{target}/dbus");
        sh.create_dir(&dbus_build_dir)?;
        cmd!(sh, "meson setup --auto-features=disabled --default-library=static --cross-file=vendor/meson-gcc-{target}.ini vendor/dbus {dbus_build_dir}").run()?;
        cmd!(sh, "meson compile -C {dbus_build_dir} dbus-1").run()?;

        let host_triple = host_triple(&sh)?;
        vec![
            format!(r#"--config=target.{host_triple}.dbus.rustc-link-lib=["dbus-1"]"#),
            format!(r#"--config=target.{target}.dbus.rustc-link-lib=["dbus-1"]"#),
            format!(r#"--config=target.{target}.dbus.rustc-link-search=["{dbus_build_dir}/dbus"]"#),
        ]
    } else {
        vec![]
    };

    // compile cirrus
    {
        let _e1 = sh.push_env("CIRRUS_VERSION", &args.version);
        let _e2 = sh.push_env("CIRRUS_BUILD_STRING", &args.build_string);
        let _e3 = sh.push_env("CIRRUS_TARGET", &target);

        let features = args.features;
        cmd!(
            sh,
            "cargo build --release --target={target} --features={features} {dbus_link_args...}"
        )
        .run()?;
        sh.copy_file(
            format!("target/{target}/release/cirrus{ext}"),
            tmp.path().join(format!("cirrus{ext}")),
        )?;
    }

    // build package
    {
        sh.create_dir("public")?;
        let pkg_path = Path::new("public").join(format!("cirrus_{target}.tar.xz"));
        package_tar_xz(&sh, tmp.path(), &pkg_path)?;
    }

    Ok(())
}

fn package_tar_xz(sh: &Shell, dir: &Path, dest: &Path) -> eyre::Result<()> {
    let mut xz = xz2::write::XzEncoder::new(std::fs::File::create(dest)?, 6);
    {
        let mut tar = tar::Builder::new(&mut xz);
        for entry in sh.read_dir(dir)? {
            let filename = entry.file_name().ok_or_else(|| eyre::eyre!("not a file"))?;
            tar.append_path_with_name(&entry, filename)?;
        }
        tar.finish()?;
    }
    xz.finish()?;
    Ok(())
}

fn host_triple(sh: &Shell) -> eyre::Result<String> {
    cmd!(sh, "rustc --version --verbose")
        .read()?
        .lines()
        .filter_map(|s| s.strip_prefix("host:"))
        .next()
        .map(|s| s.trim().to_owned())
        .ok_or_else(|| eyre::eyre!("could not find host triple"))
}
