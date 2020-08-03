use crate::model::backup;
use crate::model::repo;
use crate::secrets::RepoSecrets;
use anyhow::anyhow;
use std::path::PathBuf;
use std::process::{Child, Command};

#[derive(Debug)]
pub struct Restic {
    bin: PathBuf,
}

impl Restic {
    pub fn new(bin: impl Into<PathBuf>) -> Self {
        let bin = bin.into();
        Restic { bin }
    }

    fn run_raw<S: AsRef<str>>(
        &self,
        secrets: &RepoSecrets,
        args: impl Iterator<Item = S>,
    ) -> anyhow::Result<ResticCmd> {
        let mut cmd = Command::new(&self.bin);
        cmd.env("RESTIC_PASSWORD", &secrets.repo_password.0);
        for arg in args {
            let arg = arg.as_ref();
            cmd.arg(arg);
        }
        for (name, value) in &secrets.secrets {
            cmd.env(&name.0, &value.0);
        }
        let child = cmd.spawn()?;
        Ok(ResticCmd::new(child))
    }

    pub fn run<S: AsRef<str>>(
        &self,
        repo: &repo::Definition,
        secrets: &RepoSecrets,
        cmd: &str,
        extra_args: impl Iterator<Item = S>,
    ) -> anyhow::Result<ResticCmd> {
        let mut args = Vec::new();
        args.push("--repo".to_owned());
        args.push(repo.url.0.clone());
        args.push(cmd.to_owned());
        for arg in extra_args {
            args.push(arg.as_ref().to_owned());
        }
        self.run_raw(secrets, args.into_iter())
    }

    pub fn backup(
        &self,
        repo: &repo::Definition,
        secrets: &RepoSecrets,
        backup: &backup::Definition,
    ) -> anyhow::Result<ResticCmd> {
        let mut args = Vec::new();
        args.push(backup.path.0.clone());
        for exclude in &backup.excludes {
            args.push("--exclude".to_owned());
            args.push(exclude.0.clone());
        }
        for arg in &backup.extra_args {
            args.push(arg.clone());
        }
        self.run(repo, secrets, "backup", args.into_iter())
    }
}

#[derive(Debug)]
pub struct ResticCmd {
    process: Child,
}

impl ResticCmd {
    fn new(process: Child) -> Self {
        ResticCmd { process }
    }

    pub fn wait(mut self) -> anyhow::Result<()> {
        let status = self.process.wait()?;
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
