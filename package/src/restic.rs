use crate::download;
use xshell::*;

fn download_restic_bz2(url: &str, sha256: &str, dest_file: &str) -> eyre::Result<()> {
    download(url, dest_file)
        .expected_sha256(sha256)
        .bunzip2()
        .run()
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
            download("https://github.com/restic/restic/releases/download/v0.11.0/restic_0.11.0_windows_amd64.zip", dest_file)
                .expected_sha256("4d9ec99ceec71df88f47c5ebae5fdd15474f7d36e9685a655830c2fc89ad9153")
                .unzip_single()
                .run()
        }
        "x86_64-unknown-linux-musl" => {
            download("https://github.com/restic/restic/releases/download/v0.11.0/restic_0.11.0_linux_amd64.bz2", dest_file)
                .expected_sha256("f559e774c91f1201ffddba74d5758dec8342ad2b50a3bcd735ccb0c88839045c")
                .bunzip2()
                .run()
        }
        "armv7-unknown-linux-musleabihf" => {
            download("https://github.com/restic/restic/releases/download/v0.11.0/restic_0.11.0_linux_arm.bz2", dest_file)
                .expected_sha256("bcefbd70874b8198be4635b5c64b15359a7c28287d274e02d5177c4933ad3f71")
                .bunzip2()
                .run()
        }
        _ => eyre::bail!("unknown target {}", target),
    }
}
