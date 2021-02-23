#![windows_subsystem = "windows"]

use std::process::Command;

const CIRRUS_COMMAND: &str = "cirrus.exe";

fn main() {
    let mut cmd = set_process_options(Command::new(CIRRUS_COMMAND));
    cmd.args(std::env::args_os().skip(1)).spawn().unwrap();
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
