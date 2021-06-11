use std::{error::Error, fs::File, io::Write, path::Path};

fn write_args(workdir: &Path) -> std::io::Result<()> {
    let mut file = File::create(workdir.join("args"))?;
    for arg in std::env::args().skip(1) {
        file.write_all(arg.as_bytes())?;
        file.write_all(b"\n")?;
    }
    Ok(())
}

fn write_env(workdir: &Path) -> std::io::Result<()> {
    let mut file = File::create(workdir.join("env"))?;
    for (key, value) in std::env::vars() {
        file.write_all(key.as_bytes())?;
        file.write_all(b"=")?;
        file.write_all(value.as_bytes())?;
        file.write_all(b"\n")?;
    }
    Ok(())
}

fn get_exit_status(workdir: &Path) -> Result<i32, Box<dyn Error>> {
    let exit_status = std::fs::read_to_string(workdir.join("exit-status"))?
        .trim()
        .parse()?;
    Ok(exit_status)
}

fn copy_stdout(workdir: &Path) -> std::io::Result<()> {
    let mut file = File::open(workdir.join("stdout"))?;
    std::io::copy(&mut file, &mut std::io::stdout())?;
    Ok(())
}

fn copy_stderr(workdir: &Path) -> std::io::Result<()> {
    let mut file = File::open(workdir.join("stderr"))?;
    std::io::copy(&mut file, &mut std::io::stderr())?;
    Ok(())
}

pub fn test_binary_main() {
    let workdir = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .to_owned();
    let _ = write_args(&workdir);
    let _ = write_env(&workdir);
    let _ = copy_stdout(&workdir);
    let _ = copy_stderr(&workdir);

    let exit_status = get_exit_status(&workdir).unwrap_or(0);
    std::process::exit(exit_status);
}
