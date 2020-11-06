use cirrus_core::{appconfig::AppConfig, model::Config, restic::Restic, secrets::Secrets};
use cirrus_daemon::job;
use std::sync::Arc;

mod notifications;
mod status_icon;

#[derive(Debug, Clone)]
struct Deps {
    appconfig: Arc<AppConfig>,
    config: Arc<Config>,
    restic: Arc<Restic>,
    secrets: Arc<Secrets>,
    job_sink: cirrus_actor::ActorRef<job::Job>,
}

#[derive(Debug)]
pub struct DesktopUi {
    deps: Deps,
    notifications: notifications::Notifications,
    status_icon: Option<status_icon::StatusIcon>,
}

impl DesktopUi {
    pub fn new(
        appconfig: Arc<AppConfig>,
        config: Arc<Config>,
        restic: Arc<Restic>,
        secrets: Arc<Secrets>,
        job_sink: cirrus_actor::ActorRef<job::Job>,
    ) -> eyre::Result<Self> {
        let deps = Deps {
            appconfig,
            config,
            restic,
            secrets,
            job_sink,
        };
        let status_icon = if deps.appconfig.daemon.desktop.status_icon {
            Some(status_icon::StatusIcon::new(deps.clone())?)
        } else {
            None
        };
        Ok(Self {
            deps,
            notifications: notifications::Notifications::new()?,
            status_icon,
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
                if self.deps.appconfig.daemon.desktop.notifications.started {
                    self.notifications.job_started(&message.job)?;
                }
                if let Some(status_icon) = &mut self.status_icon {
                    status_icon.job_started(&message.job)?;
                }
            }
            job::Status::FinishedSuccessfully => {
                if self.deps.appconfig.daemon.desktop.notifications.success {
                    self.notifications.job_succeeded(&message.job)?;
                }
                if let Some(status_icon) = &mut self.status_icon {
                    status_icon.job_succeeded(&message.job)?;
                }
            }
            job::Status::FinishedWithError => {
                if self.deps.appconfig.daemon.desktop.notifications.failure {
                    self.notifications.job_failed(&message.job)?;
                }
                if let Some(status_icon) = &mut self.status_icon {
                    status_icon.job_failed(&message.job)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    async fn on_start(&mut self) -> Result<(), Self::Error> {
        if let Some(status_icon) = &mut self.status_icon {
            status_icon.start()?;
        }
        Ok(())
    }
}
