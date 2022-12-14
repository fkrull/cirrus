use super::TargetVars;
use std::path::Path;
use tempfile::TempDir;
use xshell::*;

/// Build binaries and a package.
#[derive(argh::FromArgs)]
#[argh(subcommand, name = "package")]
pub struct Args {
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
    /// cargo env file
    #[argh(option)]
    cargo_env_file: Option<String>,
}

pub fn main(args: Args) -> eyre::Result<()> {
    let sh = Shell::new()?;
    let target = args.target;
    let tmp = TempDir::new()?;
    let target_vars = TargetVars::for_target(&target)?;
    let ext = target_vars.extension;

    sh.set_var("GOOS", target_vars.go_os);
    sh.set_var("GOARCH", target_vars.go_arch);
    sh.set_var("GOARM", target_vars.go_arm.unwrap_or(""));
    sh.set_var("CIRRUS_VERSION", &args.version);
    sh.set_var("CIRRUS_BUILD_STRING", &args.build_string);
    sh.set_var("CIRRUS_TARGET", &target);

    // compile restic
    if args.build_restic {
        let bin = format!("restic{ext}");
        let bin_path = format!("../../target/{target}/{bin}");
        let _cd = sh.push_dir("vendor/restic");
        cmd!(sh, "go build -ldflags '-w -s' -o {bin_path} ./cmd/restic").run()?;
        sh.copy_file(bin_path, tmp.path().join(bin))?;
    }

    // compile cirrus
    let features = args.features;
    let cargo_env = match args.cargo_env_file {
        Some(p) => parse_env_file(&p)?,
        None => Vec::new(),
    };
    cmd!(
        sh,
        "cargo build --release --target={target} --features={features}"
    )
    .envs(cargo_env)
    .run()?;
    sh.copy_file(
        format!("target/{target}/release/cirrus{ext}"),
        tmp.path().join(format!("cirrus{ext}")),
    )?;

    // build package
    sh.create_dir("public")?;
    let pkg_path = Path::new("public").join(format!("cirrus_{target}.tar.xz"));
    package_tar_xz(&sh, tmp.path(), &pkg_path)?;

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

fn parse_env_file(path: &str) -> eyre::Result<Vec<(String, String)>> {
    let pwd = std::env::current_dir()?
        .to_str()
        .ok_or_else(|| eyre::eyre!("your current dir is not UTF-8, sorry :\\"))?
        .to_string();
    let vars = std::fs::read_to_string(path)?
        .lines()
        .map(|s| s.trim())
        .filter(|s| !s.starts_with('#'))
        .filter_map(|s| s.split_once('='))
        .filter(|(k, _)| !k.is_empty())
        .map(|(k, v)| (k.trim().to_string(), v.trim().replace("$PWD", &pwd)))
        .collect();
    Ok(vars)
}
