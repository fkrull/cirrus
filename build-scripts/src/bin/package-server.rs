use build_scripts::TargetVars;
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
}

fn main() -> eyre::Result<()> {
    let sh = Shell::new()?;
    let args: Args = argh::from_env();
    let image_arch = TargetVars::for_target(&args.target)?.image_arch;
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
            --tag=cirrus-server-image
            --file=Containerfile
            {context_path}"
    )
    .run()?;
    Ok(())
}
