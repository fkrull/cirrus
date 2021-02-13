use cirrus_actor::Messages;
use cirrus_core::model::Config;
use cirrus_daemon::{configreload::ConfigReload, daemon_config::DaemonConfig, job};
use std::sync::Arc;

mod notifications;
mod status_icon;

#[derive(Debug, Clone)]
struct Deps {
    config: Arc<Config>,
    daemon_config: Arc<DaemonConfig>,
    job_sink: Messages<job::Job>,
}

#[derive(Debug)]
pub struct DesktopUi {
    deps: Deps,
    config: Arc<Config>,
    daemon_config: Arc<DaemonConfig>,
    job_sink: Messages<job::Job>,
    notifications: notifications::Notifications,
    status_icon: Option<status_icon::StatusIcon>,
}

impl DesktopUi {
    pub fn new(
        daemon_config: Arc<DaemonConfig>,
        config: Arc<Config>,
        job_sink: Messages<job::Job>,
    ) -> eyre::Result<Self> {
        let deps = Deps {
            daemon_config: daemon_config.clone(),
            config: config.clone(),
            job_sink: job_sink.clone(),
        };
        let status_icon = if deps.daemon_config.desktop.status_icon {
            Some(status_icon::StatusIcon::new()?)
        } else {
            None
        };
        Ok(Self {
            deps,
            config,
            daemon_config,
            job_sink,
            notifications: notifications::Notifications::new()?,
            status_icon,
        })
    }

    fn handle_job_status_change(&mut self, ev: job::StatusChange) -> eyre::Result<()> {
        match ev.new_status {
            job::Status::Started => {
                if self.deps.daemon_config.desktop.notifications.started {
                    self.notifications.job_started(&ev.job)?;
                }
                if let Some(status_icon) = &mut self.status_icon {
                    status_icon.job_started(&ev.job)?;
                }
            }
            job::Status::FinishedSuccessfully => {
                if self.deps.daemon_config.desktop.notifications.success {
                    self.notifications.job_succeeded(&ev.job)?;
                }
                if let Some(status_icon) = &mut self.status_icon {
                    status_icon.job_succeeded(&ev.job)?;
                }
            }
            job::Status::FinishedWithError => {
                if self.deps.daemon_config.desktop.notifications.failure {
                    self.notifications.job_failed(&ev.job)?;
                }
                if let Some(status_icon) = &mut self.status_icon {
                    status_icon.job_failed(&ev.job)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_config_reloaded(&mut self, new_config: Arc<Config>) -> eyre::Result<()> {
        self.deps.config = new_config;
        if let Some(status_icon) = &mut self.status_icon {
            status_icon.config_reloaded(self.deps.config.clone())?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    StatusChange(job::StatusChange),
    ConfigReloaded(Arc<Config>),
}

impl From<job::StatusChange> for Message {
    fn from(ev: job::StatusChange) -> Self {
        Message::StatusChange(ev)
    }
}

impl From<ConfigReload> for Message {
    fn from(ev: ConfigReload) -> Self {
        Message::ConfigReloaded(ev.new_config)
    }
}

#[async_trait::async_trait]
impl cirrus_actor::Actor for DesktopUi {
    type Message = Message;
    type Error = eyre::Report;

    async fn on_message(&mut self, message: Self::Message) -> Result<(), Self::Error> {
        match message {
            Message::StatusChange(ev) => self.handle_job_status_change(ev)?,
            Message::ConfigReloaded(new_config) => self.handle_config_reloaded(new_config)?,
        }
        Ok(())
    }

    async fn on_start(&mut self) -> Result<(), Self::Error> {
        if let Some(status_icon) = &mut self.status_icon {
            let model = status_icon::model::Model::new(self.config.clone(), self.job_sink.clone());
            status_icon.start(model)?;
        }
        Ok(())
    }
}
