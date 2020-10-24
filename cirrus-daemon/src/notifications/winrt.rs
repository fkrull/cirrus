use crate::job::{Job, JobSpec};
use windows::ui::notifications::{
    ToastNotification, ToastNotificationManager, ToastNotifier, ToastTemplateType,
};

winrt::import!(
    dependencies
        os
    types
        windows::ui::notifications::*
);

// TODO: app ID
const APP_ID: &str =
    "{1AC14E77-02E7-4E5D-B744-2EB1AE5198B7}\\WindowsPowerShell\\v1.0\\powershell.exe";

trait WrapWinrtError {
    type Output;

    fn wrap_winrt(self) -> Self::Output;
}

impl<T> WrapWinrtError for Result<T, winrt::Error> {
    type Output = eyre::Result<T>;

    fn wrap_winrt(self) -> Self::Output {
        self.map_err(|e| eyre::eyre!("{}", e.message()))
    }
}

#[derive(Debug)]
pub(super) struct DesktopNotifications {
    notifier: ToastNotifier,
}

impl DesktopNotifications {
    pub(super) fn new() -> eyre::Result<Self> {
        let notifier = ToastNotificationManager::get_default()
            .wrap_winrt()?
            .create_toast_notifier_with_id(APP_ID)
            .wrap_winrt()?;
        Ok(DesktopNotifications { notifier })
    }

    pub(super) fn notify_job_started(&mut self, job: &Job) -> eyre::Result<()> {
        let notification = self.notification(self.started_message(job)).wrap_winrt()?;
        self.notifier.show(notification).wrap_winrt()?;
        Ok(())
    }

    pub(super) fn notify_job_succeeded(&mut self, job: &Job) -> eyre::Result<()> {
        let notification = self.notification(self.success_message(job)).wrap_winrt()?;
        self.notifier.show(notification).wrap_winrt()?;
        Ok(())
    }

    pub(super) fn notify_job_failed(&mut self, job: &Job) -> eyre::Result<()> {
        let notification = self.notification(self.failure_message(job)).wrap_winrt()?;
        self.notifier.show(notification).wrap_winrt()?;
        Ok(())
    }

    fn notification(&self, message: String) -> winrt::Result<ToastNotification> {
        let toast_xml =
            ToastNotificationManager::get_template_content(ToastTemplateType::ToastText02)?;
        let text_node = toast_xml.get_elements_by_tag_name("text")?.item(0)?;
        let text = toast_xml.create_text_node(message)?;
        text_node.append_child(text)?;
        let notification = ToastNotification::create_toast_notification(toast_xml)?;
        Ok(notification)
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
