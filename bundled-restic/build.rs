use restic_bin::*;
use std::{env::var, path::Path};

fn main() {
    let target = TargetConfig::from_env().unwrap();
    let restic_filename = restic_filename(&target);
    let restic_bin = Path::new(&var("OUT_DIR").unwrap()).join(restic_filename);
    download(&target, &restic_bin).unwrap();

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rustc-env=RESTIC_FILENAME={}", restic_filename);
    println!(
        "cargo:rustc-env=RESTIC_BIN={}",
        restic_bin.to_str().unwrap()
    );
}
