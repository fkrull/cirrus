#![windows_subsystem = "windows"]

use std::{ffi::OsString, path::PathBuf, process::Command};

fn main() {
    let cirrus_command = current_exe_dir()
        .map(|p| p.join(cirrus_exe()))
        .unwrap_or_else(|| cirrus_exe().into());

    loop {
        let cmd = Command::new(&cirrus_command);
        let mut cmd = set_process_options(cmd);
        let exit_status = cmd
            .arg("daemon")
            .args(std::env::args_os().skip(1))
            .spawn()
            .unwrap()
            .wait();
        match exit_status {
            Ok(s) if s.success() => break,
            _ => continue,
        }
    }
}

fn current_exe_dir() -> Option<PathBuf> {
    let current_exe = std::env::current_exe().ok()?;
    let dir = current_exe.parent()?;
    Some(dir.to_owned())
}

fn cirrus_exe() -> OsString {
    let mut name = OsString::from("cirrus");
    name.push(std::env::consts::EXE_SUFFIX);
    name
}

#[cfg(windows)]
fn set_process_options(mut cmd: Command) -> Command {
    use std::os::windows::process::CommandExt;
    use winapi::um::winbase;

    cmd.creation_flags(winbase::CREATE_NO_WINDOW);
    cmd
}

#[cfg(not(windows))]
fn set_process_options(cmd: Command) -> Command {
    cmd
}
