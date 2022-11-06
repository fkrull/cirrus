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

impl From<std::process::ExitStatus> for ExitStatus {
    fn from(status: std::process::ExitStatus) -> Self {
        if status.success() {
            ExitStatus::Successful
        } else {
            ExitStatus::Failed(status.code())
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
pub struct ResticProcess(pub(crate) Child);

impl ResticProcess {
    pub fn stdout(&mut self) -> &mut Option<ChildStdout> {
        &mut self.0.stdout
    }

    pub fn stderr(&mut self) -> &mut Option<ChildStderr> {
        &mut self.0.stderr
    }

    pub async fn wait(&mut self) -> Result<ExitStatus, Error> {
        self.0
            .wait()
            .await
            .map(ExitStatus::from)
            .map_err(Error::SubprocessStatusError)
    }

    pub async fn check_wait(&mut self) -> Result<(), Error> {
        self.wait().await?.check_status()
    }

    #[tracing::instrument(level = "debug", skip_all, fields(pid = self.0.id(), grace_period_secs = grace_period.as_secs_f64()))]
    pub async fn terminate(&mut self, grace_period: Duration) -> Result<(), Error> {
        tracing::debug!("trying to terminate gracefully");
        ask_to_terminate(&mut self.0)?;
        match tokio::time::timeout(grace_period, self.wait()).await {
            Ok(result) => {
                tracing::debug!("process terminated before timeout");
                result?;
            }
            Err(_) => {
                tracing::debug!("process did not terminate before timeout, killing it instead");
                self.0
                    .kill()
                    .await
                    .map_err(Error::SubprocessTerminateError)?;
            }
        };
        Ok(())
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
