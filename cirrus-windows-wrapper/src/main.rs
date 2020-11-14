#![windows_subsystem = "windows"]

use std::os::windows::process::CommandExt;
use winapi::um::winbase;

const CIRRUS_BINARY: &str = "cirrus.exe";

fn main() {
    std::process::Command::new(CIRRUS_BINARY)
        .args(std::env::args_os().skip(1))
        .creation_flags(winbase::CREATE_NO_WINDOW)
        .spawn()
        .unwrap();
}
