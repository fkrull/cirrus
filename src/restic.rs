use crate::{
    model::{backup, repo},
    secrets::RepoSecrets,
};
use anyhow::anyhow;
use std::path::PathBuf;
use tokio::process::{Child, Command};

#[derive(Debug)]
pub struct Restic {
    bin: PathBuf,
}

impl Restic {
    pub fn new(bin: impl Into<PathBuf>) -> Self {
        let bin = bin.into();
        Restic { bin }
    }

    pub fn run_raw<S: AsRef<str>>(
        &self,
        args: impl Iterator<Item = S>,
    ) -> anyhow::Result<ResticProcess> {
        let mut cmd = Command::new(&self.bin);

        for arg in args {
            cmd.arg(arg.as_ref());
        }

        let child = cmd.spawn()?;
        Ok(ResticProcess::new(child))
    }

    pub fn run<S: AsRef<str>>(
        &self,
        repo: &repo::Definition,
        secrets: &RepoSecrets,
        extra_args: impl Iterator<Item = S>,
    ) -> anyhow::Result<ResticProcess> {
        let mut cmd = Command::new(&self.bin);

        cmd.env("RESTIC_PASSWORD", &secrets.repo_password.0);
        for (name, value) in &secrets.secrets {
            cmd.env(&name.0, &value.0);
        }

        cmd.arg("--repo").arg(&repo.url.0);
        for arg in extra_args {
            cmd.arg(arg.as_ref());
        }

        let child = cmd.spawn()?;
        Ok(ResticProcess::new(child))
    }

    pub fn backup(
        &self,
        repo: &repo::Definition,
        secrets: &RepoSecrets,
        backup: &backup::Definition,
    ) -> anyhow::Result<ResticProcess> {
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
        self.run(repo, secrets, args.into_iter())
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
