use crate::job::{Job, JobSpec};
use notify_rust::Notification;

#[derive(Debug)]
pub(super) struct DesktopNotifications;

impl DesktopNotifications {
    pub(super) fn new() -> eyre::Result<Self> {
        Ok(DesktopNotifications)
    }

    pub(super) fn notify_job_started(&mut self, job: &Job) -> eyre::Result<()> {
        self.base_notification()
            .summary(&self.started_message(job))
            .show()?;
        Ok(())
    }

    pub(super) fn notify_job_succeeded(&mut self, job: &Job) -> eyre::Result<()> {
        self.base_notification()
            .summary(&self.success_message(job))
            .show()?;
        Ok(())
    }

    pub(super) fn notify_job_failed(&mut self, job: &Job) -> eyre::Result<()> {
        self.base_notification()
            .summary(&self.failure_message(job))
            .icon("dialog-error")
            .show()?;
        Ok(())
    }

    fn base_notification(&self) -> Notification {
        Notification::new()
    }

    fn started_message(&self, job: &Job) -> String {
        match &job.spec {
            JobSpec::Backup(..) => format!("Backing up '{}'", job.spec.name()),
        }
    }

    fn success_message(&self, job: &Job) -> String {
        match &job.spec {
            JobSpec::Backup(..) => format!("Backup '{}' finished successfully", job.spec.name()),
        }
    }

    fn failure_message(&self, job: &Job) -> String {
        match &job.spec {
            JobSpec::Backup(..) => format!("Backup '{}' failed", job.spec.name()),
        }
    }
}
