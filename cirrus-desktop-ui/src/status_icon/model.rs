use cirrus_actor::Messages;
use cirrus_core::model;
use cirrus_daemon::job;
use std::sync::Arc;
use std::{borrow::Cow, collections::HashMap};

#[derive(Debug, PartialEq, Clone)]
pub(super) enum Event {
    JobStarted(job::Job),
    JobSucceeded(job::Job),
    JobFailed(job::Job),

    Exit,
    UpdateConfig(Arc<model::Config>),
    RunBackup(model::backup::Name),
}

#[derive(Debug)]
pub(super) enum HandleEventOutcome {
    UpdateView,
    Unchanged,
}

#[derive(Debug)]
pub(super) enum Status {
    Idle,
    Running,
}

#[derive(Debug)]
pub(crate) struct Model {
    config: Arc<model::Config>,
    job_sink: Messages<job::Job>,
    running_jobs: HashMap<job::Id, job::Job>,
}

impl Model {
    pub(crate) fn new(config: Arc<model::Config>, job_sink: Messages<job::Job>) -> Self {
        Model {
            config,
            job_sink,
            running_jobs: HashMap::new(),
        }
    }

    pub(super) fn handle_event(&mut self, event: Event) -> eyre::Result<HandleEventOutcome> {
        match event {
            Event::JobStarted(job) => {
                self.running_jobs.insert(job.id, job);
                Ok(HandleEventOutcome::UpdateView)
            }
            Event::JobSucceeded(job) => {
                self.running_jobs.remove(&job.id);
                Ok(HandleEventOutcome::UpdateView)
            }
            Event::JobFailed(job) => {
                self.running_jobs.remove(&job.id);
                Ok(HandleEventOutcome::UpdateView)
            }
            Event::Exit => {
                std::process::exit(0);
            }
            Event::UpdateConfig(new_config) => {
                self.config = new_config;
                Ok(HandleEventOutcome::UpdateView)
            }
            Event::RunBackup(name) => {
                self.run_backup(name)?;
                Ok(HandleEventOutcome::Unchanged)
            }
        }
    }

    fn run_backup(&mut self, name: model::backup::Name) -> eyre::Result<()> {
        let backup = self
            .config
            .backups
            .get(&name)
            .ok_or_else(|| eyre::eyre!("missing backup definition '{}'", name.0))?;
        let repo = self
            .config
            .repositories
            .get(&backup.repository)
            .ok_or_else(|| {
                eyre::eyre!("missing repository definition '{:?}'", backup.repository)
            })?;
        let job = job::Job::new(
            job::BackupSpec {
                repo_name: backup.repository.clone(),
                backup_name: name,
                repo: repo.clone(),
                backup: backup.clone(),
            }
            .into(),
        );
        self.job_sink.send(job)?;
        Ok(())
    }

    pub(super) fn app_name(&self) -> &'static str {
        "Cirrus"
    }

    pub(super) fn status(&self) -> Status {
        if self.running_jobs.is_empty() {
            Status::Idle
        } else {
            Status::Running
        }
    }

    pub(super) fn status_text(&self) -> Cow<'static, str> {
        if self.running_jobs.is_empty() {
            "Idle".into()
        } else if self.running_jobs.len() == 1 {
            let job = self.running_jobs.values().next().unwrap();
            match &job.spec {
                job::Spec::Backup(_) => format!("Backing up '{}'", &job.spec.name()).into(),
            }
        } else {
            format!("Running {} jobs", self.running_jobs.len()).into()
        }
    }

    pub(super) fn tooltip(&self) -> String {
        format!("{} — {}", self.app_name(), self.status_text())
    }

    pub(super) fn backups(&self) -> impl Iterator<Item = &model::backup::Name> + '_ {
        self.config.backups.iter().map(|(name, _)| name)
    }
}
