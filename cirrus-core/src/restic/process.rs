use super::Error;
use std::time::Duration;
use tokio::process::{Child, ChildStderr, ChildStdout};

#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone)]
pub enum ExitStatus {
    Successful,
    Failed(Option<i32>),
}

impl ExitStatus {
    pub fn success(&self) -> bool {
        self == &ExitStatus::Successful
    }

    pub fn check_status(&self) -> Result<(), Error> {
        match self {
            ExitStatus::Successful => Ok(()),
            ExitStatus::Failed(_) => Err(Error::ResticError(*self)),
        }
    }

    pub fn message(&self) -> String {
        match self {
            ExitStatus::Successful => "restic exited successfully".to_owned(),
            ExitStatus::Failed(Some(code)) => {
                format!("restic exited with error status {}", code)
            }
            ExitStatus::Failed(None) => "restic exited with unknown error status".to_owned(),
        }
    }
}

#[cfg(unix)]
fn ask_to_terminate(child: &mut Child) -> Result<(), Error> {
    // TODO maybe not expect?
    let pid = child.id().expect("child should have a PID");
    unsafe { libc::kill(pid as i32, libc::SIGTERM) };
    Ok(())
}

#[cfg(not(unix))]
fn ask_to_terminate(child: &mut Child) -> Result<(), Error> {
    child
        .start_kill()
        .map_err(Error::SubprocessTerminateError)?;
    Ok(())
}

#[derive(Debug)]
pub struct ResticProcess {
    pub(crate) child: Child,
    pub(crate) extra_success_status: Option<i32>,
}

impl ResticProcess {
    pub fn stdout(&mut self) -> &mut Option<ChildStdout> {
        &mut self.child.stdout
    }

    pub fn stderr(&mut self) -> &mut Option<ChildStderr> {
        &mut self.child.stderr
    }

    pub async fn wait(&mut self) -> Result<ExitStatus, Error> {
        let proc_status = self
            .child
            .wait()
            .await
            .map_err(Error::SubprocessStatusError)?;
        if proc_status.success() || proc_status.code() == self.extra_success_status {
            Ok(ExitStatus::Successful)
        } else {
            Ok(ExitStatus::Failed(proc_status.code()))
        }
    }

    pub async fn check_wait(&mut self) -> Result<(), Error> {
        self.wait().await?.check_status()
    }

    #[tracing::instrument(level = "debug", skip_all, fields(pid = self.child.id(), grace_period_secs = grace_period.as_secs_f64()))]
    pub async fn terminate(&mut self, grace_period: Duration) -> Result<(), Error> {
        tracing::debug!("trying to terminate gracefully");
        ask_to_terminate(&mut self.child)?;
        match tokio::time::timeout(grace_period, self.wait()).await {
            Ok(result) => {
                tracing::debug!("process terminated before timeout");
                result?;
            }
            Err(_) => {
                tracing::debug!("process did not terminate before timeout, killing it instead");
                self.child
                    .kill()
                    .await
                    .map_err(Error::SubprocessTerminateError)?;
            }
        };
        Ok(())
    }
}
