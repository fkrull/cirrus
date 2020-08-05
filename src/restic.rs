use crate::{model::backup, secrets::RepoWithSecrets};
use anyhow::anyhow;
use futures::{
    future::{pending, FutureExt},
    select,
};
use std::{
    path::PathBuf,
    process::{ExitStatus, Stdio},
};
use tokio::{
    prelude::io::*,
    process::{Child, ChildStderr, ChildStdout, Command},
};

#[derive(Debug)]
pub struct Restic {
    bin: PathBuf,
}

#[derive(Debug, Default, Copy, Clone)]
pub struct Options {
    pub capture_output: bool,
}

impl Restic {
    pub fn new(bin: impl Into<PathBuf>) -> Self {
        let bin = bin.into();
        Restic { bin }
    }

    pub fn run<S: AsRef<str>>(
        &self,
        repo_with_secrets: Option<RepoWithSecrets>,
        extra_args: impl IntoIterator<Item = S>,
        options: &Options,
    ) -> anyhow::Result<ResticProcess> {
        let mut cmd = Command::new(&self.bin);

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

        let child = cmd.spawn()?;
        Ok(ResticProcess::new(child))
    }

    pub fn backup(
        &self,
        repo_with_secrets: RepoWithSecrets,
        backup: &backup::Definition,
        options: &Options,
    ) -> anyhow::Result<ResticProcess> {
        self.run(Some(repo_with_secrets), Self::backup_args(backup), options)
    }

    fn backup_args(backup: &backup::Definition) -> Vec<String> {
        let mut args = Vec::new();
        args.push("backup".to_owned());
        args.push(backup.path.0.clone());
        for exclude in &backup.excludes {
            args.push("--exclude".to_owned());
            args.push(exclude.0.clone());
        }
        for arg in &backup.extra_args {
            args.push(arg.clone());
        }
        args
    }
}

#[derive(Debug)]
pub struct ResticProcess {
    child: Child,
    stdout: Option<BufReader<ChildStdout>>,
    stderr: Option<BufReader<ChildStderr>>,
}

#[derive(Debug)]
pub enum Event {
    ProcessExit(ExitStatus),
    StdoutLine(String),
    StderrLine(String),
}

impl ResticProcess {
    fn new(mut child: Child) -> Self {
        let stdout = child.stdout.take().map(BufReader::new);
        let stderr = child.stderr.take().map(BufReader::new);
        ResticProcess {
            child,
            stdout,
            stderr,
        }
    }

    pub async fn next_event(&mut self) -> anyhow::Result<Event> {
        select! {
            event = Self::process_exit_event(&mut self.child).fuse() => event,
            event = Self::line_event(&mut self.stdout).fuse() => event,
            event = Self::line_event(&mut self.stderr).fuse() => event,
        }
    }

    async fn process_exit_event(child: &mut Child) -> anyhow::Result<Event> {
        Ok(Event::ProcessExit(child.await?))
    }

    async fn line_event(
        reader_option: &mut Option<impl AsyncBufRead + Unpin>,
    ) -> anyhow::Result<Event> {
        if let Some(reader) = reader_option {
            let mut buf = String::new();
            reader.read_line(&mut buf).await?;
            Ok(Event::StdoutLine(buf))
        } else {
            pending::<anyhow::Result<Event>>().await?;
            unreachable!()
        }
    }

    pub async fn wait(mut self) -> anyhow::Result<()> {
        loop {
            let event = self.next_event().await?;
            match event {
                Event::ProcessExit(status) => {
                    return if status.success() {
                        Ok(())
                    } else {
                        Err(anyhow!(
                            "restic exited with status {}",
                            status.code().unwrap()
                        ))
                    }
                }
                _ => continue,
            }
        }
    }
}
