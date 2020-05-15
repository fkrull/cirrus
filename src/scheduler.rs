use crate::{config::backup, App};
use chrono::{DateTime, Utc};
use log::{debug, error, info};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
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
        if let Err(err) = schedule(app.clone(), last_schedule, now, run_backup) {
            error!("scheduling failure, will retry next time: {:?}", err);
        }
        last_schedule = now;
        thread::sleep(Duration::from_secs(10));
    }
}

fn run_backup(app: Arc<App>, backup: backup::Name) {
    info!("running {}", backup.0);
    if let Some(mut job) = app.jobs.get(&backup) {
        job.finish_successful();
        app.jobs.update(job);
    }
}

fn schedule(
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
}

#[derive(Debug, PartialEq, Eq, Clone)]
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
}
