use std::path::Path;
use xshell::*;

/// Build a container image.
#[derive(argh::FromArgs)]
struct Args {
    /// rust compile target
    #[argh(option)]
    target: String,
    /// QEMU binary (must support --execve flag), may be empty
    #[argh(option)]
    qemu_binary: String,
}

fn main() -> eyre::Result<()> {
    let args: Args = argh::from_env();
    let target = args.target;

    // compile cirrus
    {
        let _e = pushenv("RUSTFLAGS", "-Clinker=rust-lld");
        cmd!("cargo build --release --target={target}").run()?;
    }

    // download restic
    build_scripts::restic(&target, "target/restic")?;

    // build container image
    let base_image = base_image(&target)?;
    let ctr = cmd!("buildah from {base_image}").read()?;

    let qemu = Some(args.qemu_binary).filter(|s| !s.is_empty());
    buildah_run(
        &ctr,
        qemu.as_ref(),
        "apk add --no-cache ca-certificates openssh-client",
    )?;
    buildah_run(&ctr, qemu.as_ref(), "mkdir -p /cache /config/cirrus")?;

    cmd!("chmod 0755 target/restic target/{target}/release/cirrus").run()?;
    cmd!("buildah copy {ctr} target/restic target/{target}/release/cirrus /usr/bin/").run()?;
    cmd!("buildah config --env XDG_CONFIG_HOME=/config {ctr}").run()?;
    cmd!("buildah config --env XDG_DATA_HOME=/data/data {ctr}").run()?;
    cmd!("buildah config --env XDG_CACHE_HOME=/data/cache {ctr}").run()?;
    cmd!("buildah config --entrypoint /usr/bin/cirrus {ctr}").run()?;
    cmd!("buildah config --volume /data {ctr}").run()?;
    cmd!("buildah commit {ctr} cirrus-server-image").run()?;

    Ok(())
}

fn base_image(target: &str) -> eyre::Result<&str> {
    Ok(match target {
        "x86_64-unknown-linux-musl" => "amd64/alpine:3.12",
        "armv7-unknown-linux-musleabihf" => "arm32v7/alpine:3.12",
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
