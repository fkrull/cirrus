use restic_bin_build::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::env::var;
    use std::path::Path;

    let target_os = var("CARGO_CFG_TARGET_OS")?;
    let target_arch = var("CARGO_CFG_TARGET_ARCH")?;
    let target_endian = var("CARGO_CFG_TARGET_ENDIAN")?;

    let out = Path::new(&var("OUT_DIR")?).join(restic_filename(&target_os));

    let urls = urls::Urls::default();

    let url_and_checksum = urls
        .url_and_checksum(&target_os, &target_arch, &target_endian)
        .unwrap();
    download::download(&url_and_checksum.url, &out)
        .decompress_mode(url_and_checksum.decompress_mode())
        .expected_sha256(&url_and_checksum.checksum)
        .run()?;

    Ok(())
}
