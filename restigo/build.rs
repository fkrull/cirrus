use std::{ffi::OsString, path::PathBuf, process::Command};

fn main() {
    println!("cargo:rerun-if-changed=src/go");
    println!("cargo:rerun-if-env-changed=GO");

    let go_cmd = std::env::var_os("GO").unwrap_or_else(|| OsString::from("go"));
    let out_dir = PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    let status = Command::new(&go_cmd)
        .current_dir("src/go")
        .arg("mod")
        .arg("download")
        .spawn()
        .unwrap()
        .wait()
        .unwrap();
    if !status.success() {
        panic!("go mod download failed");
    }
    let status = Command::new(&go_cmd)
        .current_dir("src/go")
        .arg("build")
        .arg("-o")
        .arg(out_dir.join("librestigo.a"))
        .arg("-buildmode=c-archive")
        .arg("main.go")
        .spawn()
        .unwrap()
        .wait()
        .unwrap();
    if !status.success() {
        panic!("go build failed");
    }

    println!(
        "cargo:rustc-link-search=native={}",
        out_dir.to_str().unwrap()
    );
    println!("cargo:rustc-link-lib=static=restigo");
}
