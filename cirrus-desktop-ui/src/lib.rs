use cirrus_daemon::job;
use std::sync::Arc;

mod notifications;

#[derive(Debug)]
pub struct DesktopUi {
    appconfig: Arc<cirrus_core::appconfig::AppConfig>,
    notifications: notifications::Notifications,
}

impl DesktopUi {
    pub fn new(appconfig: Arc<cirrus_core::appconfig::AppConfig>) -> eyre::Result<Self> {
        Ok(Self {
            appconfig,
            notifications: notifications::Notifications::new()?,
        })
    }
}

#[async_trait::async_trait]
impl cirrus_actor::Actor for DesktopUi {
    type Message = job::StatusChange;
    type Error = eyre::Report;

    async fn on_message(&mut self, message: Self::Message) -> Result<(), Self::Error> {
        match message.new_status {
            job::Status::Started => {
                if self.appconfig.daemon.desktop.notifications.started {
                    self.notifications.job_started(&message.job)?;
                }
            }
            job::Status::FinishedSuccessfully => {
                if self.appconfig.daemon.desktop.notifications.success {
                    self.notifications.job_succeeded(&message.job)?;
                }
            }
            job::Status::FinishedWithError => {
                if self.appconfig.daemon.desktop.notifications.failure {
                    self.notifications.job_failed(&message.job)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
}
