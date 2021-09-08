use crate::{model::backup, secrets::RepoWithSecrets};
use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
    process::Stdio,
};
use tokio::process::Command;

pub use process::*;
mod process;
mod util;

#[derive(Debug, Copy, Clone)]
pub enum Verbosity {
    None,
    V,
    VV,
    VVV,
}

impl Default for Verbosity {
    fn default() -> Self {
        Verbosity::None
    }
}

impl Verbosity {
    fn arg(&self) -> Option<&str> {
        match self {
            Verbosity::None => None,
            Verbosity::V => Some("--verbose=1"),
            Verbosity::VV => Some("--verbose=2"),
            Verbosity::VVV => Some("--verbose=3"),
        }
    }
}

#[derive(Debug, Default, Copy, Clone)]
pub struct Options {
    pub capture_output: bool,
    pub json: bool,
    pub verbose: Verbosity,
}

#[derive(Debug)]
pub struct BinaryConfig {
    pub path: PathBuf,
    pub fallback: Option<PathBuf>,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("failed to start restic process")]
    FailedToStartResticProcess(#[source] std::io::Error),
    #[error("error reading from subprocess output")]
    SubprocessIoError(#[source] std::io::Error),
    #[error("error getting subprocess status")]
    SubprocessStatusError(#[source] std::io::Error),
    #[error("{}", .0.message())]
    ResticError(ExitStatus),
    #[error("couldn't determine restic version from output")]
    FailedToGetResticVersion,
}

#[derive(Debug)]
pub struct Restic {
    binary_config: BinaryConfig,
}

impl Restic {
    #[cfg(windows)]
    const EXCLUDE_PARAM: &'static str = "--iexclude";
    #[cfg(not(windows))]
    const EXCLUDE_PARAM: &'static str = "--exclude";

    pub fn new(binary_config: BinaryConfig) -> Self {
        Restic { binary_config }
    }

    pub fn new_with_path(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        Self::new(BinaryConfig {
            path,
            fallback: None,
        })
    }

    pub fn run(
        &self,
        repo_with_secrets: Option<&RepoWithSecrets>,
        extra_args: &[impl AsRef<OsStr>],
        options: &Options,
    ) -> Result<ResticProcess, Error> {
        self.run_with_path(
            &self.binary_config.path,
            repo_with_secrets,
            extra_args,
            options,
        )
        .or_else(|e| match &self.binary_config.fallback {
            Some(fallback) => self.run_with_path(fallback, repo_with_secrets, extra_args, options),
            None => Err(e),
        })
    }

    pub fn backup(
        &self,
        repo_with_secrets: &RepoWithSecrets,
        name: &backup::Name,
        definition: &backup::Definition,
        options: &Options,
    ) -> Result<ResticProcess, Error> {
        let mut args = Vec::new();
        args.push("backup".to_owned());
        args.push(definition.path.0.clone());
        args.push("--tag".to_owned());
        args.push(format!("cirrus.{}", name.0));
        for exclude in &definition.excludes {
            args.push(Self::EXCLUDE_PARAM.to_owned());
            args.push(exclude.0.clone());
        }
        if definition.exclude_caches {
            args.push("--exclude-caches".to_owned());
        }
        for arg in &definition.extra_args {
            args.push(arg.clone());
        }

        self.run(Some(repo_with_secrets), &args, options)
    }

    fn run_with_path(
        &self,
        path: &Path,
        repo_with_secrets: Option<&RepoWithSecrets>,
        extra_args: &[impl AsRef<OsStr>],
        options: &Options,
    ) -> Result<ResticProcess, Error> {
        let mut cmd = Command::new(path);
        cmd.stdin(Stdio::null());

        if let Some(repo_with_secrets) = repo_with_secrets {
            cmd.env("RESTIC_PASSWORD", &repo_with_secrets.repo_password.0);
            for (name, value) in &repo_with_secrets.secrets {
                cmd.env(&name.0, &value.0);
            }
            cmd.arg("--repo").arg(&repo_with_secrets.repo.url.0);
        }

        for arg in extra_args {
            cmd.arg(arg.as_ref());
        }

        if options.capture_output {
            cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
        }
        if options.json {
            cmd.arg("--json");
        }
        if let Some(arg) = options.verbose.arg() {
            cmd.arg(arg);
        }

        #[cfg(windows)]
        if atty::isnt(atty::Stream::Stdout) {
            cmd.creation_flags(winapi::um::winbase::CREATE_NO_WINDOW);
        }

        let child = cmd.spawn().map_err(Error::FailedToStartResticProcess)?;
        Ok(ResticProcess::new(child))
    }
}
