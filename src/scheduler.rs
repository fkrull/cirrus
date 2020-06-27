use crate::{jobs::Job, restic, App};
use chrono::{DateTime, Utc};
use log::{debug, error, info};
use std::{
    iter,
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
    let mut previous = Utc::now();
    loop {
        let now = Utc::now();
        previous = scheduler_loop(previous, now, app.clone(), run_backup);
        thread::sleep(Duration::from_secs(10));
    }
}

fn scheduler_loop(
    previous: DateTime<Utc>,
    now: DateTime<Utc>,
    app: Arc<App>,
    mut run_fn: impl FnMut(Arc<App>, &Job),
) -> DateTime<Utc> {
    if app.pause_state.paused() {
        debug!("paused, not scheduling");
        return previous;
    }

    let scheduled_jobs = schedule(previous, now, app.jobs.jobs()).map(|job| {
        run_fn(app.clone(), &job);
        job
    });
    app.jobs.update(scheduled_jobs);
    now
}

fn run_backup(app: Arc<App>, job: &Job) {
    let f = |app: Arc<App>| -> anyhow::Result<()> {
        let repo = app
            .repositories
            .0
            .iter()
            .find(|&(name, _definition)| name == &job.definition.repository)
            .map(|(_, definition)| definition.clone());

        let repo = repo.ok_or_else(|| {
            anyhow::anyhow!("missing repo definition '{}'", job.definition.repository.0)
        })?;
        let backup = job.definition.clone();
        let name = job.name.clone();
        thread::Builder::new()
            .name(format!("backup-{}", name.0))
            .spawn(move || {
                info!("starting backup {}", name.0);
                let result = restic::init(&repo).and_then(|_| restic::backup(&repo, &backup));
                // TODO: handle errors somewhere
                let mut job = app.jobs.get(&name).unwrap();
                match result {
                    Ok(_) => {
                        info!("finished backup {}", name.0);
                        job.set_finished_successful(Utc::now());
                        app.jobs.update(iter::once(job));
                    }
                    Err(_) => {
                        error!("backup {} failed", name.0);
                        job.set_finished_failed(Utc::now());
                        app.jobs.update(iter::once(job));
                    }
                }
            })?;

        Ok(())
    };

    if let Err(err) = f(app.clone()) {
        error!("failed to schedule backup: {:?}", err);
        let mut job = job.clone();
        job.set_finished_failed(Utc::now());
        app.jobs.update(iter::once(job));
    }
}

fn schedule(
    previous: DateTime<Utc>,
    now: DateTime<Utc>,
    jobs: impl Iterator<Item = Job>,
) -> impl Iterator<Item = Job> {
    jobs.filter(|job| !job.running())
        .filter(move |job| match job.definition.next_schedule(previous) {
            Ok(Some(schedule)) if schedule <= now => true,
            Ok(_) => false,
            Err(err) => {
                error!("{}: failed to schedule: {:?}", job.name.0, err);
                false
            }
        })
        .map(move |mut job| {
            job.set_started(now);
            job
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    mod schedule {
        use super::*;
        use crate::{jobs::JobStatus, model::backup};
        use std::{iter, str::FromStr};

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

    mod scheduler_loop {
        use super::*;
        use crate::{
            jobs::{JobStatus, JobsRepo},
            model::backup,
            PauseState,
        };
        use std::str::FromStr;

        #[test]
        fn should_schedule_and_run_multiple_backups() -> anyhow::Result<()> {
            let previous: DateTime<Utc> = DateTime::from_str("2020-05-17T12:00:00Z")?;
            let now: DateTime<Utc> = DateTime::from_str("2020-05-17T13:00:00Z")?;
            let name1 = backup::Name("1".to_string());
            let definition1 = backup::Definition {
                triggers: vec![backup::Trigger::Cron {
                    cron: "15 * * * *".to_string(),
                    timezone: backup::Timezone::Utc,
                }],
                ..Default::default()
            };
            let name2 = backup::Name("1".to_string());
            let definition2 = backup::Definition {
                triggers: vec![backup::Trigger::Cron {
                    cron: "45 * * * *".to_string(),
                    timezone: backup::Timezone::Utc,
                }],
                ..Default::default()
            };
            let app = Arc::new(App {
                jobs: JobsRepo::new(
                    vec![
                        (name1.clone(), definition1.clone()),
                        (name2.clone(), definition2.clone()),
                    ]
                    .into_iter(),
                ),
                ..Default::default()
            });

            let mut jobs = Vec::new();
            let handler = |_, job: &Job| {
                jobs.push(job.clone());
            };
            let result = scheduler_loop(previous, now, app.clone(), handler);

            assert_eq!(result, now);
            assert_eq!(
                jobs,
                vec![
                    Job {
                        status: JobStatus::Running,
                        last_start: Some(now),
                        ..Job::new(name1.clone(), definition1.clone())
                    },
                    Job {
                        status: JobStatus::Running,
                        last_start: Some(now),
                        ..Job::new(name2.clone(), definition2.clone())
                    }
                ]
            );
            assert!(app.jobs.get(&name1).unwrap().running());
            assert!(app.jobs.get(&name2).unwrap().running());
            Ok(())
        }

        #[test]
        fn should_schedule_and_run_a_single_backup() -> anyhow::Result<()> {
            let previous: DateTime<Utc> = DateTime::from_str("2020-05-17T12:00:00Z")?;
            let now: DateTime<Utc> = DateTime::from_str("2020-05-17T13:00:00Z")?;
            let name1 = backup::Name("1".to_string());
            let definition1 = backup::Definition {
                triggers: vec![backup::Trigger::Cron {
                    cron: "15 13 * * *".to_string(),
                    timezone: backup::Timezone::Utc,
                }],
                ..Default::default()
            };
            let name2 = backup::Name("2".to_string());
            let definition2 = backup::Definition {
                triggers: vec![backup::Trigger::Cron {
                    cron: "45 12 * * *".to_string(),
                    timezone: backup::Timezone::Utc,
                }],
                ..Default::default()
            };
            let app = Arc::new(App {
                jobs: JobsRepo::new(
                    vec![
                        (name1.clone(), definition1.clone()),
                        (name2.clone(), definition2.clone()),
                    ]
                    .into_iter(),
                ),
                ..Default::default()
            });

            let mut jobs = Vec::new();
            let handler = |_, job: &Job| {
                jobs.push(job.clone());
            };
            let result = scheduler_loop(previous, now, app.clone(), handler);

            assert_eq!(result, now);
            assert_eq!(
                jobs,
                vec![Job {
                    status: JobStatus::Running,
                    last_start: Some(now),
                    ..Job::new(name2.clone(), definition2.clone())
                }]
            );
            assert!(!app.jobs.get(&name1).unwrap().running());
            assert!(app.jobs.get(&name2).unwrap().running());
            Ok(())
        }

        #[test]
        fn should_not_schedule_any_backup() -> anyhow::Result<()> {
            let previous: DateTime<Utc> = DateTime::from_str("2020-05-17T12:00:00Z")?;
            let now: DateTime<Utc> = DateTime::from_str("2020-05-17T13:00:00Z")?;
            let name1 = backup::Name("1".to_string());
            let definition1 = backup::Definition {
                triggers: vec![backup::Trigger::Cron {
                    cron: "* 20 * * *".to_string(),
                    timezone: backup::Timezone::Utc,
                }],
                ..Default::default()
            };
            let app = Arc::new(App {
                jobs: JobsRepo::new(vec![(name1.clone(), definition1.clone())].into_iter()),
                ..Default::default()
            });

            let mut jobs = Vec::new();
            let handler = |_, job: &Job| {
                jobs.push(job.clone());
            };
            let result = scheduler_loop(previous, now, app.clone(), handler);

            assert_eq!(result, now);
            assert_eq!(jobs, vec![]);
            assert!(!app.jobs.get(&name1).unwrap().running());
            Ok(())
        }

        #[test]
        fn should_not_schedule_any_backup_if_paused() -> anyhow::Result<()> {
            let previous: DateTime<Utc> = DateTime::from_str("2020-05-17T12:00:00Z")?;
            let now: DateTime<Utc> = DateTime::from_str("2020-05-17T13:00:00Z")?;
            let name1 = backup::Name("1".to_string());
            let definition1 = backup::Definition {
                triggers: vec![backup::Trigger::Cron {
                    cron: "30 * * * *".to_string(),
                    timezone: backup::Timezone::Utc,
                }],
                ..Default::default()
            };
            let pause = PauseState::default();
            pause.pause();
            let app = Arc::new(App {
                jobs: JobsRepo::new(vec![(name1.clone(), definition1.clone())].into_iter()),
                pause_state: pause,
                ..Default::default()
            });

            let mut jobs = Vec::new();
            let handler = |_, job: &Job| {
                jobs.push(job.clone());
            };
            let result = scheduler_loop(previous, now, app.clone(), handler);

            assert_eq!(result, previous);
            assert_eq!(jobs, vec![]);
            assert!(!app.jobs.get(&name1).unwrap().running());
            Ok(())
        }
    }
}
