use crate::jobs::Job;
use crate::App;
use chrono::{DateTime, Utc};
use log::debug;
use std::{
    sync::Arc,
    thread::{self, JoinHandle},
    time::Duration,
};

pub fn start_scheduler(app: Arc<App>) -> anyhow::Result<JoinHandle<()>> {
    let handle = thread::Builder::new()
        .name("scheduler-thread".to_string())
        .spawn(move || scheduler(app))?;
    Ok(handle)
}

fn scheduler(app: Arc<App>) {
    let mut last_schedule = Utc::now();
    loop {
        let now = Utc::now();
        debug!("running scheduler at {})", now);
        /*if let Err(err) = schedule_v1(app.clone(), last_schedule, now, run_backup) {
            error!("scheduling failure, will retry next time: {:?}", err);
        }*/
        last_schedule = now;
        thread::sleep(Duration::from_secs(10));
    }
}

/*fn run_backup(app: Arc<App>, backup: backup::Name) {
    info!("running {}", backup.0);
    if let Some(mut job) = app.jobs.get(&backup) {
        job.finish_successful();
        app.jobs.update(job);
    }
}*/

/*fn schedule_v1(
    app: Arc<App>,
    last_schedule: DateTime<Utc>,
    now: DateTime<Utc>,
    mut handler: impl FnMut(Arc<App>, backup::Name),
) -> anyhow::Result<()> {
    if app.pause_state.paused() {
        debug!("paused, not scheduling");
        return Ok(());
    }

    for (name, definition) in &app.backups.0 {
        let mut job = app.jobs.get(name).unwrap_or_else(|| Job::new(name.clone()));
        if job.running() {
            debug!("{} is running, not scheduling", name.0);
            continue;
        }
        let after = job.last_schedule.clone().unwrap_or(last_schedule);
        let matching_trigger =
            definition
                .triggers
                .iter()
                .try_find(|&trigger| -> anyhow::Result<bool> {
                    let next_schedule: DateTime<Utc> = trigger.next_schedule(&after)?;
                    let matches = next_schedule <= now;
                    if matches {
                        debug!("found matching trigger for {} at {}", name.0, next_schedule);
                        Ok(true)
                    } else {
                        Ok(false)
                    }
                })?;
        if matching_trigger.is_some() {
            info!(
                "scheduling {} (last schedule: {:?})",
                name.0, job.last_schedule
            );
            job.start_scheduled(now);
            app.jobs.update(job);
            handler(app.clone(), name.clone());
            continue;
        }
    }

    Ok(())
}*/

/*#[derive(Debug, PartialEq, Eq, Clone)]
pub enum JobStatus {
    NotRun,
    Running,
    Successful,
    Error,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Job {
    pub backup: backup::Name,
    pub status: JobStatus,
    pub last_schedule: Option<DateTime<Utc>>,
}

impl Job {
    pub fn new(backup: backup::Name) -> Self {
        Job {
            backup,
            status: JobStatus::NotRun,
            last_schedule: None,
        }
    }

    pub fn start_scheduled(&mut self, schedule: DateTime<Utc>) {
        self.status = JobStatus::Running;
        self.last_schedule = Some(schedule);
    }

    pub fn start_manual(&mut self) {
        self.status = JobStatus::Running;
    }

    pub fn finish_successful(&mut self) {
        self.status = JobStatus::Successful;
    }

    pub fn finish_error(&mut self) {
        self.status = JobStatus::Error;
    }

    pub fn running(&self) -> bool {
        self.status == JobStatus::Running
    }
}

#[derive(Debug, Default)]
pub struct JobsRepo {
    jobs: RwLock<HashMap<backup::Name, Job>>,
}

impl JobsRepo {
    pub fn get(&self, backup: &backup::Name) -> Option<Job> {
        self.jobs.read().unwrap().get(backup).cloned()
    }

    pub fn update(&self, job: Job) {
        self.jobs.write().unwrap().insert(job.backup.clone(), job);
    }
}*/

fn schedule(
    previous: DateTime<Utc>,
    now: DateTime<Utc>,
    jobs: impl Iterator<Item = Job>,
) -> impl Iterator<Item = Job> {
    jobs.filter(|job| !job.running())
        .filter(move |job| {
            let matching_trigger =
                job.definition
                    .triggers
                    .iter()
                    .try_find(|&trigger| -> anyhow::Result<bool> {
                        let next_schedule: DateTime<Utc> = trigger.next_schedule(previous)?;
                        let matches = next_schedule <= now;
                        if matches {
                            debug!(
                                "found matching trigger for {} at {}",
                                job.name.0, next_schedule
                            );
                            Ok(true)
                        } else {
                            Ok(false)
                        }
                    });
            matching_trigger.map(|t| t.is_some()).unwrap_or(false)
        })
        .map(move |mut job| {
            job.set_started(now);
            job
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jobs::JobStatus;
    use std::iter;
    use std::str::FromStr;

    #[test]
    fn should_schedule_waiting_backup() -> anyhow::Result<()> {
        let previous: DateTime<Utc> = DateTime::from_str("2020-05-16T01:05:50Z")?;
        let now: DateTime<Utc> = DateTime::from_str("2020-05-16T01:06:10Z")?;
        let job = Job::new(
            backup::Name("test".to_string()),
            backup::Definition {
                triggers: vec![backup::Trigger::Cron {
                    cron: "6 1 * * *".to_string(),
                    timezone: backup::Timezone::Utc,
                }],
                ..Default::default()
            },
        );

        let scheduled = schedule(previous, now, iter::once(job.clone())).collect::<Vec<_>>();

        assert_eq!(
            scheduled,
            vec![Job {
                status: JobStatus::Running,
                last_start: Some(now),
                ..job
            }]
        );
        Ok(())
    }

    #[test]
    fn should_schedule_failed_backup() -> anyhow::Result<()> {
        let previous: DateTime<Utc> = DateTime::from_str("2020-05-16T12:00:00Z")?;
        let now: DateTime<Utc> = DateTime::from_str("2020-05-16T13:00:00Z")?;
        let job = Job {
            status: JobStatus::FinishedWithError,
            ..Job::new(
                backup::Name("test".to_string()),
                backup::Definition {
                    triggers: vec![backup::Trigger::Cron {
                        cron: "30 * * * *".to_string(),
                        timezone: backup::Timezone::Utc,
                    }],
                    ..Default::default()
                },
            )
        };

        let scheduled = schedule(previous, now, iter::once(job.clone())).collect::<Vec<_>>();

        assert_eq!(
            scheduled,
            vec![Job {
                status: JobStatus::Running,
                last_start: Some(now),
                ..job
            }]
        );
        Ok(())
    }

    #[test]
    fn should_not_schedule_backup_thats_not_triggered() -> anyhow::Result<()> {
        let previous: DateTime<Utc> = DateTime::from_str("2020-05-16T12:00:00Z")?;
        let now: DateTime<Utc> = DateTime::from_str("2020-05-16T13:00:00Z")?;
        let job = Job::new(
            backup::Name("test".to_string()),
            backup::Definition {
                triggers: vec![backup::Trigger::Cron {
                    cron: "30 12 6 4 *".to_string(),
                    timezone: backup::Timezone::Utc,
                }],
                ..Default::default()
            },
        );

        let scheduled = schedule(previous, now, iter::once(job.clone())).collect::<Vec<_>>();

        assert_eq!(scheduled, vec![]);
        Ok(())
    }

    #[test]
    fn should_not_schedule_backup_thats_running() -> anyhow::Result<()> {
        let previous: DateTime<Utc> = DateTime::from_str("2020-05-16T12:00:00Z")?;
        let now: DateTime<Utc> = DateTime::from_str("2020-05-16T13:00:00Z")?;
        let job = Job {
            status: JobStatus::Running,
            ..Job::new(
                backup::Name("test".to_string()),
                backup::Definition {
                    triggers: vec![backup::Trigger::Cron {
                        cron: "30 * * * *".to_string(),
                        timezone: backup::Timezone::Utc,
                    }],
                    ..Default::default()
                },
            )
        };

        let scheduled = schedule(previous, now, iter::once(job.clone())).collect::<Vec<_>>();

        assert_eq!(scheduled, vec![]);
        Ok(())
    }
}
