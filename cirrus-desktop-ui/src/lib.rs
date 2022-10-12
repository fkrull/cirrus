use cirrus_core::config::Config;
use cirrus_daemon::{config_reload::ConfigReload, job, suspend::Suspend};
use shindig::{Events, Subscriber};
use std::sync::Arc;

mod status_icon;

#[derive(Debug)]
pub struct StatusIcon {
    model: status_icon::Model,
    sub_status_change: Subscriber<job::StatusChange>,
    sub_config_reload: Subscriber<ConfigReload>,
    sub_suspend: Subscriber<Suspend>,
}

impl StatusIcon {
    pub fn new(config: Arc<Config>, mut events: Events, suspend: Suspend) -> eyre::Result<Self> {
        status_icon::Handle::check()?;
        let sub_status_change = events.subscribe();
        let sub_config_reload = events.subscribe();
        let sub_suspend = events.subscribe();
        let model = status_icon::Model::new(config, events, suspend);
        Ok(StatusIcon {
            model,
            sub_status_change,
            sub_config_reload,
            sub_suspend,
        })
    }

    /*fn handle_job_status_change(&mut self, ev: job::StatusChange) -> eyre::Result<()> {
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
    }*/

    pub async fn run(mut self) -> eyre::Result<()> {
        let mut handle = status_icon::Handle::start(self.model)?;
        loop {
            let event = tokio::select! {
                status_change = self.sub_status_change.recv() => status_icon::Event::JobStatusChange(status_change?),
                config_reload = self.sub_config_reload.recv() => status_icon::Event::ConfigReload(config_reload?),
                suspend = self.sub_suspend.recv() => status_icon::Event::Suspend(suspend?),
            };
            handle.send(event)?;
        }
    }
}
