#[cfg(feature = "download")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use restic_bin::{download, restic_filename, TargetConfig};

    #[derive(Debug, thiserror::Error)]
    enum CliError {
        #[error("specify a target triple")]
        MissingTargetTriple,
    }

    let triple = std::env::args()
        .nth(1)
        .ok_or(CliError::MissingTargetTriple)?;
    let target = TargetConfig::from_triple(triple)?;
    let filename = restic_filename(&target);
    download(&target, filename)?;
    println!("downloaded {} for target {}", filename, target);
    Ok(())
}

#[cfg(not(feature = "download"))]
fn main() {
    panic!("crate was built without the 'download' feature");
}
