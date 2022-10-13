use cirrus_core::config::Config;
use cirrus_daemon::{config_reload::ConfigReload, job, suspend::Suspend};
use std::sync::Arc;

mod model;
use events::{Events, Subscriber};
pub(crate) use model::*;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
use windows as platform_specific;

#[cfg(unix)]
mod xdg;
#[cfg(unix)]
use xdg as platform_specific;

#[derive(Debug)]
pub struct StatusIcon {
    model: Model,
    sub_status_change: Subscriber<job::StatusChange>,
    sub_config_reload: Subscriber<ConfigReload>,
    sub_suspend: Subscriber<Suspend>,
}

impl StatusIcon {
    pub fn new(config: Arc<Config>, events: &mut Events, suspend: Suspend) -> eyre::Result<Self> {
        platform_specific::check()?;
        let model = Model::new(config, events.sender(), suspend);
        Ok(StatusIcon {
            model,
            sub_status_change: events.subscribe(),
            sub_config_reload: events.subscribe(),
            sub_suspend: events.subscribe(),
        })
    }

    pub async fn run(mut self) -> eyre::Result<()> {
        let mut handle = platform_specific::start(self.model)?;
        loop {
            let event = tokio::select! {
                status_change = self.sub_status_change.recv() => Event::JobStatusChange(status_change?),
                config_reload = self.sub_config_reload.recv() => Event::ConfigReload(config_reload?),
                suspend = self.sub_suspend.recv() => Event::Suspend(suspend?),
            };
            handle.send(event)?;
        }
    }
}
