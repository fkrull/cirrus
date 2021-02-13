#![allow(clippy::transmute_ptr_to_ptr)] // for winrt::import!

use crate::job;
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
pub(crate) struct Notifications {
    notifier: ToastNotifier,
}

impl Notifications {
    pub(crate) fn new() -> eyre::Result<Self> {
        let notifier = ToastNotificationManager::get_default()
            .wrap_winrt()?
            .create_toast_notifier_with_id(APP_ID)
            .wrap_winrt()?;
        Ok(Notifications { notifier })
    }

    pub(crate) fn job_started(&mut self, job: &job::Job) -> eyre::Result<()> {
        let notification = self.notification(self.started_message(job)).wrap_winrt()?;
        self.notifier.show(notification).wrap_winrt()?;
        Ok(())
    }

    pub(crate) fn job_succeeded(&mut self, job: &job::Job) -> eyre::Result<()> {
        let notification = self.notification(self.success_message(job)).wrap_winrt()?;
        self.notifier.show(notification).wrap_winrt()?;
        Ok(())
    }

    pub(crate) fn job_failed(&mut self, job: &job::Job) -> eyre::Result<()> {
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

    fn started_message(&self, job: &job::Job) -> String {
        match &job.spec {
            job::Spec::Backup(b) => format!("Backing up '{}'", b.name()),
        }
    }

    fn success_message(&self, job: &job::Job) -> String {
        match &job.spec {
            job::Spec::Backup(b) => format!("Backup '{}' finished successfully", b.name()),
        }
    }

    fn failure_message(&self, job: &job::Job) -> String {
        match &job.spec {
            job::Spec::Backup(b) => format!("Backup '{}' failed", b.name()),
        }
    }
}
