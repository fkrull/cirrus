use crate::{model::backup, secrets::RepoWithSecrets};
use eyre::Context as _;
use futures::{prelude::*, stream::BoxStream};
use std::{ffi::OsStr, path::PathBuf, process::Stdio};
use tokio::{
    io::{AsyncBufReadExt as _, BufReader},
    process::{Child, Command},
};
use tokio_stream::wrappers::LinesStream;

#[derive(Debug)]
pub struct Restic {
    binary: Option<PathBuf>,
}

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

impl Restic {
    #[cfg(windows)]
    const EXCLUDE_PARAM: &'static str = "--iexclude";
    #[cfg(not(windows))]
    const EXCLUDE_PARAM: &'static str = "--exclude";

    pub fn new(binary: Option<PathBuf>) -> Self {
        Restic { binary }
    }

    pub fn run<S: AsRef<OsStr>>(
        &self,
        repo_with_secrets: Option<RepoWithSecrets>,
        extra_args: impl IntoIterator<Item = S>,
        options: &Options,
    ) -> eyre::Result<ResticProcess> {
        let extra_args = extra_args.into_iter().collect::<Vec<_>>();
        let child = if let Some(binary) = &self.binary {
            run_internal(
                binary.as_os_str(),
                repo_with_secrets.as_ref(),
                &extra_args,
                options,
            )
        } else {
            run_internal(
                OsStr::new("restic"),
                repo_with_secrets.as_ref(),
                &extra_args,
                options,
            )
            .or_else(|e| {
                let bundled_restic_exe = current_exe_dir()
                    .map(|p| {
                        p.join("restic")
                            .with_extension(std::env::consts::EXE_EXTENSION)
                    })
                    .ok_or(e)?;
                run_internal(
                    bundled_restic_exe.as_os_str(),
                    repo_with_secrets.as_ref(),
                    &extra_args,
                    options,
                )
            })
        };

        Ok(ResticProcess::new(child?))
    }

    pub fn backup(
        &self,
        repo_with_secrets: RepoWithSecrets,
        name: &backup::Name,
        definition: &backup::Definition,
        options: &Options,
    ) -> eyre::Result<ResticProcess> {
        self.run(
            Some(repo_with_secrets),
            Self::backup_args(name, definition),
            options,
        )
    }

    fn backup_args(name: &backup::Name, definition: &backup::Definition) -> Vec<String> {
        let mut args = Vec::new();
        args.push("backup".to_owned());
        args.push(definition.path.0.clone());
        args.push("--tag".to_owned());
        args.push(format!("cirrus-backup-{}", name.0));
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
        args
    }
}

fn current_exe_dir() -> Option<PathBuf> {
    let current_exe = std::env::current_exe().ok()?;
    let dir = current_exe.parent()?;
    Some(dir.to_owned())
}

fn run_internal(
    program: &OsStr,
    repo_with_secrets: Option<&RepoWithSecrets>,
    extra_args: &[impl AsRef<OsStr>],
    options: &Options,
) -> eyre::Result<Child> {
    let mut cmd = Command::new(program);
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

    cmd.spawn().wrap_err("failed to start restic process")
}

#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone)]
pub enum ExitStatus {
    Successful,
    Failed(Option<i32>),
}

impl ExitStatus {
    pub fn success(&self) -> bool {
        self == &ExitStatus::Successful
    }

    pub fn check_status(&self) -> eyre::Result<()> {
        match self {
            ExitStatus::Successful => Ok(()),
            ExitStatus::Failed(Some(code)) => {
                Err(eyre::eyre!("restic exited with status {}", code))
            }
            ExitStatus::Failed(None) => Err(eyre::eyre!("restic exited with unknown status")),
        }
    }
}

impl From<std::process::ExitStatus> for ExitStatus {
    fn from(status: std::process::ExitStatus) -> Self {
        if status.success() {
            ExitStatus::Successful
        } else {
            ExitStatus::Failed(status.code())
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum Event {
    StdoutLine(String),
    StderrLine(String),
}

fn merge_output_streams(child: &'_ mut Child) -> BoxStream<'static, std::io::Result<Event>> {
    let stdout = child
        .stdout
        .take()
        .map(|io| LinesStream::new(BufReader::new(io).lines()).map_ok(Event::StdoutLine));
    let stderr = child
        .stderr
        .take()
        .map(|io| LinesStream::new(BufReader::new(io).lines()).map_ok(Event::StderrLine));

    match (stdout, stderr) {
        (Some(stdout), Some(stderr)) => Box::pin(stream::select(stdout, stderr)),
        (Some(stdout), None) => Box::pin(stdout),
        (None, Some(stderr)) => Box::pin(stderr),
        (None, None) => Box::pin(stream::empty()),
    }
}

#[pin_project::pin_project]
pub struct ResticProcess {
    child: Child,
    #[pin]
    events: BoxStream<'static, std::io::Result<Event>>,
}

impl std::fmt::Debug for ResticProcess {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResticProcess")
            .field("child", &self.child)
            .field("events", &"<...>")
            .finish()
    }
}

impl ResticProcess {
    fn new(mut child: Child) -> Self {
        let events = merge_output_streams(&mut child);
        ResticProcess { child, events }
    }

    pub async fn wait(&mut self) -> std::io::Result<ExitStatus> {
        while let Some(_) = self.next().await {}
        self.child.wait().await.map(ExitStatus::from)
    }

    pub async fn check_wait(&mut self) -> eyre::Result<()> {
        self.wait().await?.check_status()
    }
}

impl Stream for ResticProcess {
    type Item = std::io::Result<Event>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.project().events.poll_next(cx)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.events.size_hint()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod exit_status {
        use super::*;

        #[test]
        fn should_be_ok_for_successful_exit_status() {
            assert!(ExitStatus::Successful.check_status().is_ok());
        }

        #[test]
        fn should_be_err_for_failed_exit_status() {
            assert!(ExitStatus::Failed(Some(1)).check_status().is_err());
        }

        #[test]
        fn should_be_err_for_failed_exit_status_without_code() {
            assert!(ExitStatus::Failed(None).check_status().is_err());
        }
    }
}
