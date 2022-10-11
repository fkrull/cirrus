use cirrus_core::config;
use cirrus_daemon::shutdown::RequestShutdown;
use cirrus_daemon::{job, suspend};
use eyre::WrapErr;
use shindig::Events;
use std::{borrow::Cow, collections::HashMap, sync::Arc};

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub(crate) use windows::StatusIcon;

#[cfg(target_family = "unix")]
mod xdg;
#[cfg(target_family = "unix")]
pub(crate) use xdg::StatusIcon;

// TODO: split into internal and external events
#[derive(Debug, PartialEq, Clone)]
pub(crate) enum Event {
    JobStarted(job::Job),
    JobSucceeded(job::Job),
    JobFailed(job::Job),
    JobCancelled(job::Job),
    Suspended,
    Unsuspended,

    ToggleSuspended,
    UpdateConfig(Arc<config::Config>),
    RunBackup(config::backup::Name),
    OpenConfigFile,
    Exit,
}

#[derive(Debug)]
pub(crate) struct Model {
    config: Arc<config::Config>,
    events: Events,
    running_jobs: HashMap<job::Id, job::Job>,
    suspended: bool,
}

impl Model {
    pub(crate) fn new(config: Arc<config::Config>, events: Events) -> Self {
        Model {
            config,
            events,
            running_jobs: HashMap::new(),
            suspended: false,
        }
    }

    fn handle_event(&mut self, event: Event) -> eyre::Result<HandleEventOutcome> {
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
            Event::JobCancelled(job) => {
                self.running_jobs.remove(&job.id);
                Ok(HandleEventOutcome::UpdateView)
            }
            Event::Suspended => {
                self.suspended = true;
                Ok(HandleEventOutcome::UpdateView)
            }
            Event::Unsuspended => {
                self.suspended = false;
                Ok(HandleEventOutcome::UpdateView)
            }

            Event::ToggleSuspended => {
                self.toggle_suspend();
                Ok(HandleEventOutcome::Unchanged)
            }
            Event::UpdateConfig(new_config) => {
                self.config = new_config;
                Ok(HandleEventOutcome::UpdateView)
            }
            Event::RunBackup(name) => {
                self.run_backup(name)?;
                Ok(HandleEventOutcome::Unchanged)
            }
            Event::OpenConfigFile => {
                self.open_config_file()?;
                Ok(HandleEventOutcome::Unchanged)
            }
            Event::Exit => {
                tracing::info!("exiting due to user request via status icon");
                self.events.send(RequestShutdown);
                Ok(HandleEventOutcome::Unchanged)
            }
        }
    }

    fn run_backup(&mut self, name: config::backup::Name) -> eyre::Result<()> {
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
        self.events.send(job);
        Ok(())
    }

    fn open_config_file(&self) -> eyre::Result<()> {
        let config_path = self
            .config
            .source
            .as_ref()
            .ok_or_else(|| eyre::Report::msg("configuration not loaded from file"))?;
        opener::open(config_path)
            .wrap_err_with(|| format!("failed to open config file {}", config_path.display()))
    }

    fn toggle_suspend(&mut self) {
        if self.suspended {
            self.events.send(suspend::Unsuspend);
        } else {
            self.events.send(suspend::Suspend::UntilDisabled);
        }
    }

    fn app_name(&self) -> &'static str {
        "Cirrus"
    }

    fn status(&self) -> Status {
        if self.suspended {
            Status::Suspended
        } else if !self.running_jobs.is_empty() {
            Status::Running
        } else {
            Status::Idle
        }
    }

    fn status_text(&self) -> Cow<'static, str> {
        if self.suspended {
            "Suspended".into()
        } else if self.running_jobs.is_empty() {
            "Idle".into()
        } else if self.running_jobs.len() == 1 {
            let job = self.running_jobs.values().next().unwrap();
            match &job.spec {
                job::Spec::Backup(b) => format!("Backing up '{}'", b.name()).into(),
            }
        } else {
            format!("Running {} jobs", self.running_jobs.len()).into()
        }
    }

    fn tooltip(&self) -> String {
        format!("{} — {}", self.app_name(), self.status_text())
    }

    fn backups(&self) -> impl Iterator<Item = &config::backup::Name> + '_ {
        self.config.backups.iter().map(|(name, _)| name)
    }

    fn can_open_config_file(&self) -> bool {
        self.config.source.is_some()
    }
}

#[derive(Debug)]
enum HandleEventOutcome {
    UpdateView,
    Unchanged,
}

#[derive(Debug)]
enum Status {
    Idle,
    Running,
    Suspended,
}
