use crate::job::{JobStatus, JobStatusChange};

#[cfg(all(windows, feature = "desktop-notifications"))]
mod winrt;
#[cfg(all(windows, feature = "desktop-notifications"))]
use self::winrt::DesktopNotifications;

#[derive(Debug)]
pub struct Notifications {
    #[cfg(feature = "desktop-notifications")]
    desktop_notifications: DesktopNotifications,
}

impl Notifications {
    pub fn new() -> eyre::Result<Self> {
        Ok(Notifications {
            #[cfg(feature = "desktop-notifications")]
            desktop_notifications: DesktopNotifications::new()?,
        })
    }
}

#[async_trait::async_trait]
impl cirrus_actor::Actor for Notifications {
    type Message = JobStatusChange;
    type Error = eyre::Report;

    async fn on_message(&mut self, message: Self::Message) -> Result<(), Self::Error> {
        match message.new_status {
            JobStatus::Started => {
                #[cfg(feature = "desktop-notifications")]
                self.desktop_notifications
                    .notify_job_started(&message.job)?;
            }
            JobStatus::FinishedSuccessfully => {
                #[cfg(feature = "desktop-notifications")]
                self.desktop_notifications
                    .notify_job_succeeded(&message.job)?;
            }
            JobStatus::FinishedWithError => {
                #[cfg(feature = "desktop-notifications")]
                self.desktop_notifications.notify_job_failed(&message.job)?;
            }
            _ => {}
        }
        Ok(())
    }
}
