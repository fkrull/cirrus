use crate::{config::backup, secrets::RepoWithSecrets};
use std::{ffi::OsStr, path::PathBuf, process::Stdio};
use tokio::process::Command;

use crate::tag::Tag;
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
pub enum Output {
    #[default]
    Null,
    Inherit,
    Capture,
}

impl From<Output> for Stdio {
    fn from(v: Output) -> Self {
        match v {
            Output::Null => Stdio::null(),
            Output::Inherit => Stdio::inherit(),
            Output::Capture => Stdio::piped(),
        }
    }
}

#[derive(Debug, Default, Copy, Clone)]
pub struct Options {
    pub stdout: Output,
    pub stderr: Output,
    pub json: bool,
    pub verbose: Verbosity,
}

impl Options {
    pub fn inherit_output() -> Options {
        Options {
            stdout: Output::Inherit,
            stderr: Output::Inherit,
            ..Default::default()
        }
    }
}

#[derive(Debug)]
pub struct CommandConfig {
    pub path: PathBuf,
    pub env_var: Option<&'static str>,
}

impl CommandConfig {
    pub fn from_path(path: PathBuf) -> Self {
        CommandConfig {
            path,
            env_var: None,
        }
    }

    pub fn with_env_var(self, env_var: &'static str) -> Self {
        Self {
            env_var: Some(env_var),
            ..self
        }
    }

    fn to_command(&self) -> Command {
        let mut cmd = Command::new(&self.path);
        if let Some(env_var) = &self.env_var {
            cmd.env(env_var, "1");
        }
        cmd
    }
}

#[derive(Debug)]
pub struct Config {
    pub primary: CommandConfig,
    pub fallback: Option<CommandConfig>,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("failed to start restic process")]
    FailedToStartResticProcess(#[source] std::io::Error),
    #[error("error reading from subprocess output")]
    SubprocessIoError(#[source] std::io::Error),
    #[error("error getting subprocess status")]
    SubprocessStatusError(#[source] std::io::Error),
    #[error("error killing process")]
    SubprocessTerminateError(#[source] std::io::Error),
    #[error("{}", .0.message())]
    ResticError(ExitStatus),
    #[error("couldn't determine restic version from output")]
    FailedToGetResticVersion,
}

#[derive(Debug)]
pub struct Restic {
    config: Config,
}

impl Restic {
    #[cfg(windows)]
    const EXCLUDE_PARAM: &'static str = "--iexclude";
    #[cfg(not(windows))]
    const EXCLUDE_PARAM: &'static str = "--exclude";

    pub fn new(config: Config) -> Self {
        Restic { config }
    }

    pub fn new_with_path(path: impl Into<PathBuf>) -> Self {
        Self::new(Config {
            primary: CommandConfig::from_path(path.into()),
            fallback: None,
        })
    }

    pub fn run(
        &self,
        repo_with_secrets: Option<&RepoWithSecrets>,
        extra_args: &[impl AsRef<OsStr>],
        options: &Options,
    ) -> Result<ResticProcess, Error> {
        self.run_with_config(&self.config.primary, repo_with_secrets, extra_args, options)
            .or_else(|e| match &self.config.fallback {
                Some(fallback) => {
                    self.run_with_config(fallback, repo_with_secrets, extra_args, options)
                }
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
        let mut args = vec![
            "backup".to_owned(),
            definition.path.0.clone(),
            "--tag".to_owned(),
            Tag::for_backup(name).0,
        ];
        for exclude in &definition.excludes {
            args.push(Self::EXCLUDE_PARAM.to_owned());
            args.push(exclude.0.clone());
        }
        if definition.exclude_caches {
            args.push("--exclude-caches".to_owned());
        }
        if let Some(exclude_larger_than) = &definition.exclude_larger_than {
            args.push("--exclude-larger-than".to_owned());
            args.push(exclude_larger_than.clone());
        }
        for arg in &definition.extra_args {
            args.push(arg.clone());
        }

        self.run(Some(repo_with_secrets), &args, options)
    }

    fn run_with_config(
        &self,
        config: &CommandConfig,
        repo_with_secrets: Option<&RepoWithSecrets>,
        extra_args: &[impl AsRef<OsStr>],
        options: &Options,
    ) -> Result<ResticProcess, Error> {
        let mut cmd = config.to_command();
        cmd.stdin(Stdio::null())
            .stdout(options.stdout)
            .stderr(options.stderr)
            // kill-on-drop is a final fallback, normally the process gets terminated gracefully
            .kill_on_drop(true);

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
        Ok(ResticProcess(child))
    }
}
