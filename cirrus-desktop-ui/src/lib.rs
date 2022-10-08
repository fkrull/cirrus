use cirrus_core::config::Config;
use cirrus_daemon::{configreload::ConfigReload, job};
use shindig::Events;
use std::sync::Arc;

mod status_icon;

#[derive(Debug)]
pub struct DesktopUi {
    config: Arc<Config>,
    events: Events,
    status_icon: status_icon::StatusIcon,
}

impl DesktopUi {
    pub fn new(config: Arc<Config>, events: Events) -> eyre::Result<Self> {
        let status_icon = status_icon::StatusIcon::new()?;
        Ok(Self {
            config,
            events,
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

    pub async fn run(&mut self) -> eyre::Result<()> {
        let model = status_icon::Model::new(self.config.clone(), self.events.clone());
        self.status_icon.start(model)?;
        let mut status_change_recv = self.events.subscribe::<job::StatusChange>();
        let mut config_reload_recv = self.events.subscribe::<ConfigReload>();
        loop {
            tokio::select! {
                status_change = status_change_recv.recv() => self.handle_job_status_change(status_change?)?,
                config_reload = config_reload_recv.recv() => self.handle_config_reloaded(config_reload?.new_config)?,
            }
        }
    }
}
