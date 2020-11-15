use package::download;
use std::path::Path;
use xshell::*;

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
    /// base image (must be Alpine)
    #[argh(option)]
    base_image: String,
    /// QEMU binary (must support --execve flag)
    #[argh(option)]
    qemu_binary: Option<String>,
}

fn main() -> eyre::Result<()> {
    let args: Args = argh::from_env();
    let target = args.target;

    // compile cirrus
    cmd!("cargo build --release --target={target}").run()?;

    // download restic
    download(args.restic_url, "target/restic.bz2")
        .expected_sha256(args.restic_sha256)
        .run()?;
    cmd!("bunzip2 target/restic.bz2").run()?;
    cmd!("chmod 0755 target/restic").run()?;

    // build container image
    let base_image = args.base_image;
    let ctr = cmd!("buildah from {base_image}").read()?;

    buildah_run(
        &ctr,
        args.qemu_binary.as_ref(),
        "apk add --no-cache ca-certificates openssh-client",
    )?;
    buildah_run(
        &ctr,
        args.qemu_binary.as_ref(),
        "mkdir -p /cache /config/cirrus",
    )?;

    cmd!("buildah copy {ctr} target/restic target/{target}/release/cirrus /usr/bin/").run()?;
    cmd!("buildah config --env XDG_CONFIG_HOME=/config {ctr}").run()?;
    cmd!("buildah config --env XDG_CACHE_HOME=/cache {ctr}").run()?;
    cmd!("buildah config --entrypoint /usr/bin/cirrus {ctr}").run()?;
    cmd!("buildah config --volume /cache {ctr}").run()?;
    cmd!("buildah commit {ctr} cirrus-container-image").run()?;

    Ok(())
}

fn buildah_run(ctr: &str, qemu_binary: Option<&String>, script: &str) -> eyre::Result<()> {
    if let Some(qemu_binary) = qemu_binary {
        let qemu_binary = Path::new(qemu_binary).canonicalize()?;
        let dir = qemu_binary.parent().unwrap();
        let file = qemu_binary.file_name().unwrap();
        cmd!("buildah run -v {dir}:/qemu {ctr} -- /qemu/{file} --execve /bin/sh -c {script}")
            .run()?;
    } else {
        cmd!("buildah run {ctr} -- /bin/sh -c {script}").run()?;
    }
    Ok(())
}
