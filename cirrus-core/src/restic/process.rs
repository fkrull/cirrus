use futures::{prelude::*, stream::BoxStream};
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
    pub(super) fn new(mut child: Child) -> Self {
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
