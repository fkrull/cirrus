use cirrus_daemon::job;

#[derive(Debug)]
pub(crate) struct StatusIcon {}

impl StatusIcon {
    pub(crate) fn new() -> eyre::Result<Self> {
        Ok(StatusIcon {})
    }

    pub(crate) fn start(&mut self) -> eyre::Result<()> {
        todo!()
    }

    pub(crate) fn job_started(&mut self, job: &job::Job) -> eyre::Result<()> {
        todo!()
    }

    pub(crate) fn job_succeeded(&mut self, job: &job::Job) -> eyre::Result<()> {
        todo!()
    }

    pub(crate) fn job_failed(&mut self, job: &job::Job) -> eyre::Result<()> {
        todo!()
    }
}
