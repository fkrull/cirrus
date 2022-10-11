use super::Error;
use futures::{prelude::*, stream::BoxStream};
use std::time::Duration;
use tokio::{
    io::{AsyncBufReadExt as _, BufReader},
    process::Child,
};
use tokio_stream::wrappers::LinesStream;

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

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum Event {
    StdoutLine(String),
    StderrLine(String),
}

fn merge_output_streams(child: &'_ mut Child) -> BoxStream<'static, Result<Event, Error>> {
    let stdout = child.stdout.take().map(|io| {
        LinesStream::new(BufReader::new(io).lines())
            .map_ok(Event::StdoutLine)
            .map_err(Error::SubprocessIoError)
    });
    let stderr = child.stderr.take().map(|io| {
        LinesStream::new(BufReader::new(io).lines())
            .map_ok(Event::StderrLine)
            .map_err(Error::SubprocessIoError)
    });

    match (stdout, stderr) {
        (Some(stdout), Some(stderr)) => Box::pin(stream::select(stdout, stderr)),
        (Some(stdout), None) => Box::pin(stdout),
        (None, Some(stderr)) => Box::pin(stderr),
        (None, None) => Box::pin(stream::empty()),
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

#[pin_project::pin_project]
pub struct ResticProcess {
    child: Child,
    #[pin]
    events: BoxStream<'static, Result<Event, Error>>,
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
    pub(super) fn new(mut child: Child) -> Self {
        let events = merge_output_streams(&mut child);
        ResticProcess { child, events }
    }

    pub async fn wait(&mut self) -> Result<ExitStatus, Error> {
        while let Some(_) = self.next().await {}
        self.child
            .wait()
            .await
            .map(ExitStatus::from)
            .map_err(Error::SubprocessStatusError)
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

impl Stream for ResticProcess {
    type Item = Result<Event, Error>;

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
