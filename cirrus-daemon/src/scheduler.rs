use crate::job;
use cirrus_actor::{Actor, Messages};
use cirrus_core::config;
use std::{collections::HashMap, sync::Arc, time::Duration};
use time::PrimitiveDateTime;
use tracing::info;

const SCHEDULE_INTERVAL: Duration = Duration::from_secs(30);

#[derive(Debug)]
pub struct Scheduler {
    config: Arc<config::Config>,
    job_sink: Messages<job::Job>,
    start_time: time::OffsetDateTime,
    previous_schedules: HashMap<config::backup::Name, time::OffsetDateTime>,
}

impl Scheduler {
    pub fn new(config: Arc<config::Config>, job_sink: Messages<job::Job>) -> Self {
        Scheduler {
            config,
            job_sink,
            start_time: time::OffsetDateTime::now_utc(),
            previous_schedules: HashMap::new(),
        }
    }

    fn run_schedules(&mut self) -> eyre::Result<()> {
        use crate::job::BackupSpec;

        let now = time::OffsetDateTime::now_local()?;
        let now_local = PrimitiveDateTime::new(now.date(), now.time());
        let backups_to_schedule = self
            .config
            .backups
            .iter()
            .filter(|(_, definition)| !definition.disable_triggers)
            .map(|(name, definition)| -> eyre::Result<_> {
                let prev = self.previous_schedules.get(name).copied();
                let next = definition.next_schedule(prev.unwrap_or(self.start_time))?;
                Ok((name, definition, next))
            })
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .filter_map(|(name, definition, next)| next.map(|next| (name, definition, next)))
            .filter(|(_, _, next)| next.0 <= now_local);

        for (name, backup, _) in backups_to_schedule {
            let repo = self
                .config
                .repositories
                .get(&backup.repository)
                .ok_or_else(|| {
                    eyre::eyre!("missing repository definition '{}'", backup.repository.0)
                })?;
            let backup_job = job::Job::new(
                BackupSpec {
                    repo_name: backup.repository.clone(),
                    backup_name: name.clone(),
                    repo: repo.clone(),
                    backup: backup.clone(),
                }
                .into(),
            );
            info!("scheduling '{}'", backup_job.spec.label());
            self.job_sink.send(backup_job)?;
            self.previous_schedules.insert(name.clone(), now);
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    ConfigReloaded(Arc<config::Config>),
    SchedulerTick,
}

impl From<crate::configreload::ConfigReload> for Message {
    fn from(ev: crate::configreload::ConfigReload) -> Self {
        Message::ConfigReloaded(ev.new_config)
    }
}

#[async_trait::async_trait]
impl Actor for Scheduler {
    type Message = Message;
    type Error = eyre::Report;

    async fn on_message(&mut self, message: Self::Message) -> Result<(), Self::Error> {
        match message {
            Message::ConfigReloaded(new_config) => self.config = new_config,
            Message::SchedulerTick => self.run_schedules()?,
        }
        Ok(())
    }

    async fn idle(&mut self) -> Result<Self::Message, Self::Error> {
        tokio::time::sleep(SCHEDULE_INTERVAL).await;
        Ok(Message::SchedulerTick)
    }
}
