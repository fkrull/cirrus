use cirrus_actor::Messages;
use cirrus_core::model::Config;
use cirrus_daemon::{configreload::ConfigReload, daemon_config::DaemonConfig, job};
use std::sync::Arc;

mod status_icon;

#[derive(Debug)]
pub struct DesktopUi {
    config: Arc<Config>,
    daemon_config: Arc<DaemonConfig>,
    job_sink: Messages<job::Job>,
    status_icon: Option<status_icon::StatusIcon>,
}

impl DesktopUi {
    pub fn new(
        daemon_config: Arc<DaemonConfig>,
        config: Arc<Config>,
        job_sink: Messages<job::Job>,
    ) -> eyre::Result<Self> {
        let status_icon = if daemon_config.desktop.status_icon {
            Some(status_icon::StatusIcon::new()?)
        } else {
            None
        };
        Ok(Self {
            config,
            daemon_config,
            job_sink,
            status_icon,
        })
    }

    fn handle_job_status_change(&mut self, ev: job::StatusChange) -> eyre::Result<()> {
        match ev.new_status {
            job::Status::Started => {
                if let Some(status_icon) = &mut self.status_icon {
                    status_icon.job_started(&ev.job)?;
                }
            }
            job::Status::FinishedSuccessfully => {
                if let Some(status_icon) = &mut self.status_icon {
                    status_icon.job_succeeded(&ev.job)?;
                }
            }
            job::Status::FinishedWithError => {
                if let Some(status_icon) = &mut self.status_icon {
                    status_icon.job_failed(&ev.job)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_config_reloaded(&mut self, new_config: Arc<Config>) -> eyre::Result<()> {
        self.config = new_config;
        if let Some(status_icon) = &mut self.status_icon {
            status_icon.config_reloaded(self.config.clone())?;
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
            let model = status_icon::Model::new(
                self.config.clone(),
                self.job_sink.clone(),
                self.daemon_config.versions.restic_version.clone(),
            );
            status_icon.start(model)?;
        }
        Ok(())
    }
}
