use std::path::{Path, PathBuf};

const RESTIC_VERSION: &str = "v0.12.1-0.20220107210826-0eade4aeaefd";

fn main() {
    println!("cargo:rerun-if-changed=go.mod");
    println!("cargo:rerun-if-changed=go.sum");
    if Path::new("vendor").is_dir() {
        println!("cargo:rerun-if-changed=vendor/");
    }

    let is_msvc = std::env::var("TARGET").unwrap().ends_with("-msvc");
    if is_msvc {
        panic!("This crate does not work on -msvc targets (cgo only supports gcc on Windows");
    }

    let out_dir = PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    gobuild::Build::new()
        .file("main.go")
        .env("GOCACHE", out_dir.join("go-cache"))
        .ldflags(format!("-X 'github.com/restic/restic/cmd/restic.version={}'", RESTIC_VERSION))
        .trim_paths(true)
        .compile("restigo");
}
