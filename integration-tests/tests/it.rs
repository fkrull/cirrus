use assert_cmd::cargo::cargo_bin;
use std::{
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
};

mod cirrus_core;

pub struct EnvVarGuard {
    keys: Vec<OsString>,
}

impl EnvVarGuard {
    pub fn new() -> Self {
        Self { keys: vec![] }
    }

    pub fn with_var(mut self, key: &str, value: impl AsRef<OsStr>) -> Self {
        std::env::set_var(key, value);
        self.keys.push(OsString::from(key));
        self
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        for key in &self.keys {
            std::env::remove_var(key);
        }
    }
}

fn exe_name(name: &str) -> String {
    format!("{}{}", name, std::env::consts::EXE_SUFFIX)
}

pub struct Workdir {
    dir: tempfile::TempDir,
}

impl Workdir {
    pub fn new() -> std::io::Result<Self> {
        let dir = tempfile::TempDir::new()?;
        std::fs::copy(
            cargo_bin("integration-tests"),
            dir.path().join(exe_name("test_binary")),
        )?;
        Ok(Self { dir })
    }

    pub fn with_exit_status(self, exit_status: i32) -> std::io::Result<Self> {
        std::fs::write(self.path().join("exit-status"), exit_status.to_string())?;
        Ok(self)
    }

    pub fn path(&self) -> &Path {
        self.dir.path()
    }

    pub fn bin(&self) -> PathBuf {
        self.dir.path().join(exe_name("test_binary"))
    }
}

pub struct Args {
    args: Vec<String>,
}

impl Args {
    pub fn assert_args(&self, args: &[impl AsRef<str>]) -> &Self {
        let args = args.into_iter().map(|s| s.as_ref()).collect::<Vec<_>>();
        self.assert_args_(&args);
        self
    }

    fn assert_args_(&self, args: &[&str]) {
        assert_eq!(&self.args, args);
    }
}

pub fn parse_args(workdir: &Path) -> std::io::Result<Args> {
    let args = std::fs::read_to_string(workdir.join("args"))?
        .lines()
        .map(|s| s.to_owned())
        .collect();
    Ok(Args { args })
}

pub struct Env {
    env: Vec<(String, String)>,
}

impl Env {
    pub fn assert_var(&self, key: impl AsRef<str>, value: impl AsRef<str>) -> &Self {
        self.assert_var_(key.as_ref(), value.as_ref());
        self
    }

    fn assert_var_(&self, key: &str, value: &str) {
        assert!(self
            .env
            .iter()
            .find(|(k, v)| k == key && v == value)
            .is_some());
    }
}

pub fn parse_env(workdir: &Path) -> std::io::Result<Env> {
    let env = std::fs::read_to_string(workdir.join("env"))?
        .lines()
        .filter_map(|s| s.split_once('='))
        .map(|(key, value)| (key.to_owned(), value.to_owned()))
        .collect();
    Ok(Env { env })
}
