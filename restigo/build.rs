use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=go.mod");
    println!("cargo:rerun-if-changed=go.sum");
    println!("cargo:rerun-if-changed=vendor/");

    let out_dir = PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    gobuild::Build::new()
        .file("main.go")
        .env("GOCACHE", out_dir.join("go-cache"))
        .compile("restigo");
}
