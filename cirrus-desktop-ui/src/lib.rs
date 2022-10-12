use cirrus_core::config::Config;
use cirrus_daemon::{config_reload::ConfigReload, job, suspend::Suspend};
use shindig::Events;
use std::sync::Arc;

mod status_icon;

#[derive(Debug)]
pub struct DesktopUi {
    config: Arc<Config>,
    events: Events,
    initial_suspend: Suspend,
    status_icon: status_icon::StatusIcon,
}

impl DesktopUi {
    pub fn new(
        config: Arc<Config>,
        events: Events,
        initial_suspend: Suspend,
    ) -> eyre::Result<Self> {
        let status_icon = status_icon::StatusIcon::new()?;
        Ok(Self {
            config,
            events,
            initial_suspend,
            status_icon,
        })
    }

    fn handle_job_status_change(&mut self, ev: job::StatusChange) -> eyre::Result<()> {
        let event = match ev.new_status {
            job::Status::Started => status_icon::Event::JobStarted(ev.job),
            job::Status::FinishedSuccessfully => status_icon::Event::JobSucceeded(ev.job),
            job::Status::FinishedWithError => status_icon::Event::JobFailed(ev.job),
            job::Status::Cancelled => status_icon::Event::JobCancelled(ev.job),
        };
        self.status_icon.send(event)
    }

    fn handle_config_reloaded(&mut self, new_config: Arc<Config>) -> eyre::Result<()> {
        self.config = new_config;
        self.status_icon
            .send(status_icon::Event::UpdateConfig(self.config.clone()))
    }

    fn handle_suspend(&mut self, suspend: Suspend) -> eyre::Result<()> {
        self.status_icon.send(status_icon::Event::Suspend(suspend))
    }

    pub async fn run(&mut self) -> eyre::Result<()> {
        let model = status_icon::Model::new(
            self.config.clone(),
            self.events.clone(),
            self.initial_suspend.clone(),
        );
        self.status_icon.start(model)?;
        let mut status_change_recv = self.events.subscribe::<job::StatusChange>();
        let mut config_reload_recv = self.events.subscribe::<ConfigReload>();
        let mut suspend_recv = self.events.subscribe::<Suspend>();
        loop {
            tokio::select! {
                status_change = status_change_recv.recv() => self.handle_job_status_change(status_change?)?,
                config_reload = config_reload_recv.recv() => self.handle_config_reloaded(config_reload?.new_config)?,
                suspend = suspend_recv.recv() => self.handle_suspend(suspend?)?,
            }
        }
    }
}
