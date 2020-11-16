use crate::download;
use xshell::*;

fn download_restic_bz2_linux(url: &str, sha256: &str, dest_file: &str) -> eyre::Result<()> {
    let bz2_dest_file = format!("{}.bz2", dest_file);
    download(url, &bz2_dest_file)
        .expected_sha256(sha256)
        .run()?;
    cmd!("bunzip2 {bz2_dest_file}").run()?;
    cmd!("chmod 0755 {dest_file}").run()?;
    Ok(())
}

fn download_restic_zip(url: &str, sha256: &str, dest_file: &str) -> eyre::Result<()> {
    download(url, dest_file)
        .expected_sha256(sha256)
        .unzip_single()
        .run()
}

pub fn restic(target: &str, dest_file: &str) -> eyre::Result<()> {
    match target {
        "x86_64-pc-windows-msvc" => {
            download_restic_zip(
                "https://github.com/restic/restic/releases/download/v0.11.0/restic_0.11.0_windows_amd64.zip",
                "4d9ec99ceec71df88f47c5ebae5fdd15474f7d36e9685a655830c2fc89ad9153",
                dest_file
            )?;
        }
        "x86_64-unknown-linux-musl" => {
            download_restic_bz2_linux(
                "https://github.com/restic/restic/releases/download/v0.11.0/restic_0.11.0_linux_amd64.bz2",
                "f559e774c91f1201ffddba74d5758dec8342ad2b50a3bcd735ccb0c88839045c",
                dest_file
            )?;
        }
        "armv7-unknown-linux-musleabihf" => {
            download_restic_bz2_linux(
                "https://github.com/restic/restic/releases/download/v0.11.0/restic_0.11.0_linux_arm.bz2", 
                "bcefbd70874b8198be4635b5c64b15359a7c28287d274e02d5177c4933ad3f71",
                dest_file
            )?;
        }
        _ => eyre::bail!("unknown target {}", target),
    };
    Ok(())
}
