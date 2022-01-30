use cirrus_actor::Messages;
use cirrus_core::model::Config;
use cirrus_daemon::{configreload::ConfigReload, job};
use std::sync::Arc;

mod status_icon;

#[derive(Debug)]
pub struct DesktopUi {
    config: Arc<Config>,
    job_sink: Messages<job::Job>,
    status_icon: status_icon::StatusIcon,
}

impl DesktopUi {
    pub fn new(config: Arc<Config>, job_sink: Messages<job::Job>) -> eyre::Result<Self> {
        let status_icon = status_icon::StatusIcon::new()?;
        Ok(Self {
            config,
            job_sink,
            status_icon,
        })
    }

    fn handle_job_status_change(&mut self, ev: job::StatusChange) -> eyre::Result<()> {
        match ev.new_status {
            job::Status::Started => {
                self.status_icon.job_started(&ev.job)?;
            }
            job::Status::FinishedSuccessfully => {
                self.status_icon.job_succeeded(&ev.job)?;
            }
            job::Status::FinishedWithError => {
                self.status_icon.job_failed(&ev.job)?;
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_config_reloaded(&mut self, new_config: Arc<Config>) -> eyre::Result<()> {
        self.config = new_config;
        self.status_icon.config_reloaded(self.config.clone())?;
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
        let model = status_icon::Model::new(self.config.clone(), self.job_sink.clone());
        self.status_icon.start(model)?;
        Ok(())
    }
}
