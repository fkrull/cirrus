use crate::{model::backup, secrets::RepoWithSecrets};
use anyhow::anyhow;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::{Child, Command};

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
    process: Child,
}

impl ResticProcess {
    fn new(process: Child) -> Self {
        ResticProcess { process }
    }

    pub async fn wait(self) -> anyhow::Result<()> {
        let status = self.process.await?;
        if status.success() {
            Ok(())
        } else {
            Err(anyhow!(
                "restic exited with status {}",
                status.code().unwrap()
            ))
        }
    }
}
