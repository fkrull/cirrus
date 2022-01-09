use std::{fs::File, path::Path};
use tempfile::TempDir;
use xshell::*;
use zip::ZipArchive;

/// Build a container image.
#[derive(argh::FromArgs)]
struct Args {
    /// zip file of binaries
    #[argh(positional)]
    binaries_zip: String,

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

    // initialise container image
    let base_image = base_image(&args.target)?;
    let ctr = cmd!("buildah from {base_image}").read()?;

    // copy files
    let tmp = TempDir::new()?;
    let mut zip = ZipArchive::new(File::open(args.binaries_zip)?)?;
    zip.extract(tmp.path())?;
    for path in read_dir(tmp.path())? {
        cmd!("buildah copy --chown root:root {ctr} {path} /usr/bin/").run()?;
    }

    // setup image
    buildah_run(
        &ctr,
        qemu.as_ref(),
        "apt-get update && apt-get install --no-install-recommends -y ca-certificates libdbus-1-3 openssh-client && apt-get clean && rm -rf /var/lib/apt",
    )?;
    cmd!("buildah config --env XDG_CONFIG_HOME=/config {ctr}").run()?;
    cmd!("buildah config --env XDG_DATA_HOME=/data/data {ctr}").run()?;
    cmd!("buildah config --env XDG_CACHE_HOME=/data/cache {ctr}").run()?;
    cmd!("buildah config --entrypoint '[\"/usr/bin/cirrus\"]' {ctr}").run()?;
    cmd!("buildah config --cmd '[\"daemon\"]' {ctr}").run()?;
    cmd!("buildah config --volume /data {ctr}").run()?;
    cmd!("buildah commit {ctr} cirrus-server-image").run()?;

    Ok(())
}

fn base_image(target: &str) -> eyre::Result<&str> {
    Ok(match target {
        "x86_64-unknown-linux-gnu" => "docker.io/amd64/debian:11-slim",
        "armv7-unknown-linux-gnueabihf" => "docker.io/arm32v7/debian:11-slim",
        "aarch64-unknown-linux-gnu" => "docker.io/arm64v8/debian:11-slim",
        _ => eyre::bail!("unknown target {}", target),
    })
}

fn buildah_run(ctr: &str, qemu_binary: Option<&String>, script: &str) -> eyre::Result<()> {
    if let Some(qemu_binary) = qemu_binary {
        let qemu_binary = Path::new(qemu_binary).canonicalize()?;
        cmd!("buildah run -v {qemu_binary}:/qemu {ctr} -- /qemu --execve /bin/sh -c {script}")
            .run()?;
    } else {
        cmd!("buildah run {ctr} -- /bin/sh -c {script}").run()?;
    }
    Ok(())
}
