use crate::config_reload::ConfigReload;
use crate::job;
use cirrus_core::config;
use shindig::{EventsBuilder, Sender, Subscriber};
use std::{collections::HashMap, sync::Arc, time::Duration};
use time::PrimitiveDateTime;

const SCHEDULE_INTERVAL: Duration = Duration::from_secs(30);

#[derive(Debug)]
pub struct Scheduler {
    config: Arc<config::Config>,
    sender: Sender,
    sub_config_reload: Subscriber<ConfigReload>,
    start_time: time::OffsetDateTime,
    previous_schedules: HashMap<config::backup::Name, time::OffsetDateTime>,
}

impl Scheduler {
    pub fn new(config: Arc<config::Config>, events: &mut EventsBuilder) -> Self {
        Scheduler {
            config,
            sender: events.sender(),
            sub_config_reload: events.subscribe(),
            start_time: time::OffsetDateTime::now_utc(),
            previous_schedules: HashMap::new(),
        }
    }

    #[tracing::instrument(level = "debug")]
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
            tracing::info!(label = backup_job.spec.label(), "scheduling backup",);
            self.sender.send(backup_job);
            self.previous_schedules.insert(name.clone(), now);
        }

        Ok(())
    }

    pub async fn run(&mut self) -> eyre::Result<()> {
        loop {
            tokio::select! {
                config_reload = self.sub_config_reload.recv() => self.config = config_reload?.new_config,
                _ = tokio::time::sleep(SCHEDULE_INTERVAL) => self.run_schedules()?
            }
        }
    }
}
