use crate::job;
use notify_rust::Notification;

#[derive(Debug)]
pub(crate) struct Notifications;

impl Notifications {
    pub(crate) fn new() -> eyre::Result<Self> {
        Ok(Notifications)
    }

    pub(crate) fn notify_job_started(&mut self, job: &job::Job) -> eyre::Result<()> {
        self.base_notification()
            .summary(&self.started_message(job))
            .show()?;
        Ok(())
    }

    pub(crate) fn notify_job_succeeded(&mut self, job: &job::Job) -> eyre::Result<()> {
        self.base_notification()
            .summary(&self.success_message(job))
            .show()?;
        Ok(())
    }

    pub(crate) fn notify_job_failed(&mut self, job: &job::Job) -> eyre::Result<()> {
        self.base_notification()
            .summary(&self.failure_message(job))
            .icon("dialog-error")
            .show()?;
        Ok(())
    }

    fn base_notification(&self) -> Notification {
        Notification::new()
    }

    fn started_message(&self, job: &job::Job) -> String {
        match &job.spec {
            job::Spec::Backup(..) => format!("Backing up '{}'", job.spec.name()),
        }
    }

    fn success_message(&self, job: &job::Job) -> String {
        match &job.spec {
            job::Spec::Backup(..) => format!("Backup '{}' finished successfully", job.spec.name()),
        }
    }

    fn failure_message(&self, job: &job::Job) -> String {
        match &job.spec {
            job::Spec::Backup(..) => format!("Backup '{}' failed", job.spec.name()),
        }
    }
}
