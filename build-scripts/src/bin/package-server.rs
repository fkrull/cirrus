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
    let sh = Shell::new()?;
    let args: Args = argh::from_env();
    let qemu = args
        .qemu_binary
        .filter(|s| !s.is_empty())
        .unwrap_or_else(no_qemu);
    let image_arch = image_arch(&args.target)?;
    let binaries_tar = Path::new(&args.binaries_tar);
    let context_path = binaries_tar
        .parent()
        .ok_or_else(|| eyre::eyre!("no parent path"))?;
    let tarball = binaries_tar
        .file_name()
        .ok_or_else(|| eyre::eyre!("no file name"))?;
    cmd!(
        sh,
        "buildah build-using-dockerfile
            --build-arg=IMAGE_ARCH={image_arch}
            --build-arg=TARBALL={tarball}
            --volume={qemu}:/qemu:z,ro
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

fn no_qemu() -> String {
    const CARGO_MANIFEST_DIR: &str = env!("CARGO_MANIFEST_DIR");
    format!("{}/no-qemu.sh", CARGO_MANIFEST_DIR)
}
