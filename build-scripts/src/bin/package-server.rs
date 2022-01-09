use std::path::Path;
use xshell::*;

/// Build a container image.
#[derive(argh::FromArgs)]
struct Args {
    /// tar file of binaries
    #[argh(positional)]
    binaries_tar: String,

    /// rust compile target
    #[argh(option)]
    target: String,
    /// QEMU binary (must support --execve flag), may be empty
    #[argh(option)]
    qemu_binary: Option<String>,
}

fn main() -> eyre::Result<()> {
    let args: Args = argh::from_env();
    let qemu = args.qemu_binary.filter(|s| !s.is_empty());
    let qemu_args = qemu_build_args(qemu.as_ref());
    let image_arch = image_arch(&args.target)?;
    let binaries_tar = Path::new(&args.binaries_tar);
    let context_path = binaries_tar
        .parent()
        .ok_or_else(|| eyre::eyre!("no parent path"))?;
    let tarball = binaries_tar
        .file_name()
        .ok_or_else(|| eyre::eyre!("no file name"))?;
    cmd!(
        "buildah build
            {qemu_args...}
            --build-arg=IMAGE_ARCH={image_arch}
            --build-arg=TARBALL={tarball}
            --tag=cirrus-server-image
            --file=Containerfile
            {context_path}"
    )
    .run()?;
    Ok(())
}

fn image_arch(target: &str) -> eyre::Result<&str> {
    Ok(match target {
        "x86_64-unknown-linux-gnu" => "amd64",
        "armv7-unknown-linux-gnueabihf" => "arm32v7",
        "aarch64-unknown-linux-gnu" => "arm64v8",
        _ => eyre::bail!("unknown target {}", target),
    })
}

fn qemu_build_args(qemu_binary: Option<&String>) -> Vec<String> {
    if let Some(qemu_binary) = qemu_binary {
        vec![
            format!("--volume={}:/qemu", qemu_binary),
            "--build-arg=QEMU=/qemu".to_owned(),
        ]
    } else {
        vec!["--build-arg=QEMU=".to_owned()]
    }
}
