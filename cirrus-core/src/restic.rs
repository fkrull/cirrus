use crate::{model::backup, secrets::RepoWithSecrets};
use eyre::{eyre, Context};
use futures::future::pending;
use std::ffi::OsStr;
use std::{
    path::PathBuf,
    process::{ExitStatus, Stdio},
};
use tokio::{
    io::*,
    process::{Child, ChildStderr, ChildStdout, Command},
};

#[derive(Debug)]
pub struct Restic {
    bin: PathBuf,
}

#[derive(Debug, Default, Copy, Clone)]
pub struct Options {
    pub capture_output: bool,
    pub json: bool,
}

impl Restic {
    #[cfg(windows)]
    const EXCLUDE_PARAM: &'static str = "--iexclude";
    #[cfg(not(windows))]
    const EXCLUDE_PARAM: &'static str = "--exclude";

    pub fn new(bin: impl Into<PathBuf>) -> Self {
        let bin = bin.into();
        Restic { bin }
    }

    pub fn run<S: AsRef<OsStr>>(
        &self,
        repo_with_secrets: Option<RepoWithSecrets>,
        extra_args: impl IntoIterator<Item = S>,
        options: &Options,
    ) -> eyre::Result<ResticProcess> {
        let mut cmd = Command::new(&self.bin);
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

        #[cfg(windows)]
        if atty::isnt(atty::Stream::Stdout) {
            cmd.creation_flags(winapi::um::winbase::CREATE_NO_WINDOW);
        }

        let child = cmd.spawn().wrap_err("failed to start restic process")?;
        Ok(ResticProcess::new(child))
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

    pub async fn next_event(&mut self) -> eyre::Result<Event> {
        tokio::select! {
            status = self.child.wait() => Ok(Event::ProcessExit(status?)),
            line = Self::maybe_read_line(&mut self.stdout) => Ok(Event::StdoutLine(line?)),
            line = Self::maybe_read_line(&mut self.stderr) => Ok(Event::StderrLine(line?)),
        }
    }

    async fn maybe_read_line(
        maybe_reader: &mut Option<impl AsyncBufRead + Unpin>,
    ) -> std::io::Result<String> {
        if let Some(reader) = maybe_reader {
            let mut buf = String::new();
            loop {
                reader.read_line(&mut buf).await?;
                if !buf.is_empty() {
                    buf.truncate(buf.trim_end().len());
                    break Ok(buf);
                }
            }
        } else {
            pending::<std::io::Result<String>>().await
        }
    }

    pub async fn wait(mut self) -> eyre::Result<()> {
        loop {
            let event = self.next_event().await?;
            match event {
                Event::ProcessExit(status) => {
                    return if status.success() {
                        Ok(())
                    } else if let Some(code) = status.code() {
                        Err(eyre!("restic exited with status {}", code))
                    } else {
                        Err(eyre!("restic exited with unknown status"))
                    }
                }
                _ => continue,
            }
        }
    }

    pub async fn kill(&mut self) {
        self.child.kill().await.ok();
    }
}
