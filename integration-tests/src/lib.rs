use std::{
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
};

mod test_binary_main;
pub use test_binary_main::test_binary_main;

// Adapted from
// https://github.com/rust-lang/cargo/blob/485670b3983b52289a2f353d589c57fae2f60f82/tests/testsuite/support/mod.rs#L507
// https://github.com/assert-rs/assert_cmd/blob/3ae01c9cf76e8b652c8ed4d2d64ff53149096339/src/cargo.rs#L192
fn target_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .map(|mut path| {
            path.pop();
            if path.ends_with("deps") {
                path.pop();
            }
            path
        })
        .unwrap()
}

fn exe_name(name: &str) -> String {
    format!("{}{}", name, std::env::consts::EXE_SUFFIX)
}

fn cargo_bin(name: &str) -> PathBuf {
    target_dir().join(exe_name(name))
}

fn copy_or_symlink(src: &Path, dest: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(src, dest)
    }

    #[cfg(not(unix))]
    {
        std::fs::copy(src, dest).map(|_| ())
    }
}

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

pub struct Workdir {
    dir: tempfile::TempDir,
}

impl Workdir {
    const TARGET_BINARY_NAME: &'static str = "test-binary";

    pub fn new(binary_name: &str) -> Self {
        let dir = tempfile::TempDir::new().unwrap();
        copy_or_symlink(
            &cargo_bin(binary_name),
            &dir.path().join(exe_name(Self::TARGET_BINARY_NAME)),
        )
        .unwrap();
        Self { dir }
    }

    pub fn with_exit_status(self, exit_status: i32) -> Self {
        std::fs::write(self.path().join("exit-status"), exit_status.to_string()).unwrap();
        self
    }

    pub fn with_stdout(self, stdout: impl AsRef<[u8]>) -> Self {
        std::fs::write(self.path().join("stdout"), stdout.as_ref()).unwrap();
        self
    }

    pub fn with_stderr(self, stderr: impl AsRef<[u8]>) -> Self {
        std::fs::write(self.path().join("stderr"), stderr.as_ref()).unwrap();
        self
    }

    pub fn with_file(self, name: &str, contents: impl AsRef<[u8]>) -> Self {
        std::fs::write(self.path().join(name), contents.as_ref()).unwrap();
        self
    }

    pub fn path(&self) -> &Path {
        self.dir.path()
    }

    pub fn test_binary(&self) -> PathBuf {
        self.dir.path().join(exe_name(Self::TARGET_BINARY_NAME))
    }

    pub fn args(&self) -> Args {
        Args::new(&self.path().join("args")).unwrap()
    }

    pub fn env(&self) -> Env {
        Env::new(&self.path().join("env")).unwrap()
    }
}

pub struct Args {
    args: Vec<String>,
}

impl Args {
    fn new(args_file: &Path) -> std::io::Result<Args> {
        let args = std::fs::read_to_string(args_file)?
            .lines()
            .map(|s| s.to_owned())
            .collect();
        Ok(Args { args })
    }

    pub fn assert_args(&self, args: &[impl AsRef<str>]) -> &Self {
        let args = args.into_iter().map(|s| s.as_ref()).collect::<Vec<_>>();
        assert_eq!(&self.args, &args);
        self
    }
}

pub struct Env {
    env: Vec<(String, String)>,
}

impl Env {
    fn new(env_file: &Path) -> std::io::Result<Self> {
        let env = std::fs::read_to_string(env_file)?
            .lines()
            .filter_map(|s| s.split_once('='))
            .map(|(key, value)| (key.to_owned(), value.to_owned()))
            .collect();
        Ok(Env { env })
    }

    pub fn assert_var(&self, key: impl AsRef<str>, value: impl AsRef<str>) -> &Self {
        let key = key.as_ref();
        let value = value.as_ref();
        assert!(self
            .env
            .iter()
            .find(|(k, v)| k == key && v == value)
            .is_some());
        self
    }
}
