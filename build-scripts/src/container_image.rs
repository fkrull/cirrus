use super::TargetVars;
use std::path::Path;
use xshell::*;

/// Build a container image.
#[derive(argh::FromArgs)]
#[argh(subcommand, name = "container-image")]
pub struct Args {
    /// tar file of binaries
    #[argh(option)]
    binaries_tar: String,
    /// image base tag (without OS and arch)
    #[argh(option)]
    tag: String,
    /// rust compile target
    #[argh(option)]
    target: String,
}

pub fn main(args: Args) -> eyre::Result<()> {
    let sh = Shell::new()?;
    let binaries_tar = Path::new(&args.binaries_tar);
    let target_vars = TargetVars::for_target(&args.target)?;
    let container_arch = target_vars.container_arch;
    let tag = args.tag;
    let context_path = binaries_tar
        .parent()
        .ok_or_else(|| eyre::eyre!("no parent path"))?;
    let tarball = binaries_tar
        .file_name()
        .ok_or_else(|| eyre::eyre!("no file name"))?;
    cmd!(
        sh,
        "podman build
            --build-arg=TARBALL={tarball}
            --arch={container_arch}
            --tag={tag}
            --file=Dockerfile
            {context_path}"
    )
    .run()?;
    Ok(())
}
