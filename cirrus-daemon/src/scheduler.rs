use crate::job;
use chrono::DateTime;
use cirrus_core::{model, restic::Restic, secrets::Secrets};
use eyre::eyre;
use log::info;
use std::{collections::HashMap, sync::Arc, time::Duration};

const SCHEDULE_INTERVAL: Duration = Duration::from_secs(30);

#[derive(Debug)]
pub struct Scheduler {
    config: Arc<model::Config>,
    restic: Arc<Restic>,
    secrets: Arc<Secrets>,
    job_queues: cirrus_actor::ActorRef<job::Job>,

    start_time: DateTime<chrono::Utc>,
    previous_schedules: HashMap<model::backup::Name, DateTime<chrono::Utc>>,
}

impl Scheduler {
    pub fn new(
        config: Arc<model::Config>,
        restic: Arc<Restic>,
        secrets: Arc<Secrets>,
        job_queues: cirrus_actor::ActorRef<job::Job>,
    ) -> Self {
        Scheduler {
            config,
            restic,
            secrets,
            job_queues,
            start_time: chrono::Utc::now(),
            previous_schedules: HashMap::new(),
        }
    }

    pub async fn run(&mut self) -> eyre::Result<()> {
        use crate::job::BackupSpec;
        use tokio::time::sleep;

        loop {
            let now = chrono::Utc::now();
            let backups_to_schedule = self
                .config
                .backups
                .iter()
                .map(|(name, definition)| -> eyre::Result<_> {
                    let prev = self.previous_schedules.get(name).copied();
                    let next = definition.next_schedule(prev.unwrap_or(self.start_time))?;
                    Ok((name, definition, next))
                })
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .filter_map(|(name, definition, next)| next.map(|next| (name, definition, next)))
                .filter(|(_, _, next)| next <= &now);

            for (name, backup, _) in backups_to_schedule {
                let repo = self
                    .config
                    .repositories
                    .get(&backup.repository)
                    .ok_or_else(|| {
                        eyre!("missing repository definition '{}'", backup.repository.0)
                    })?;
                let backup_job = job::Job::new(
                    BackupSpec {
                        restic: self.restic.clone(),
                        secrets: self.secrets.clone(),
                        repo_name: backup.repository.clone(),
                        backup_name: name.clone(),
                        repo: repo.clone(),
                        backup: backup.clone(),
                    }
                    .into(),
                );
                info!("scheduling backup '{}'", backup_job.spec.name());
                self.job_queues.send(backup_job)?;
                self.previous_schedules.insert(name.clone(), now);
            }

            sleep(SCHEDULE_INTERVAL).await;
        }
    }
}
