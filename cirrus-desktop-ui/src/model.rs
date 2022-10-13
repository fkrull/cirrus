use cirrus_core::config;
use cirrus_daemon::{
    config_reload::ConfigReload, job, shutdown::RequestShutdown, suspend::Suspend,
};
use events::Sender;
use eyre::WrapErr;
use std::{borrow::Cow, collections::HashMap, sync::Arc};

#[derive(Debug, PartialEq, Clone)]
pub(crate) enum Event {
    JobStatusChange(job::StatusChange),
    Suspend(Suspend),
    ConfigReload(ConfigReload),

    ToggleSuspended,
    RunBackup(config::backup::Name),
    OpenConfigFile,
    Exit,
}

#[derive(Debug)]
pub(crate) enum HandleEventOutcome {
    UpdateView,
    Unchanged,
}

#[derive(Debug)]
pub(crate) enum Status {
    Idle,
    Running,
    Suspended,
}

#[derive(Debug)]
pub(crate) struct Model {
    config: Arc<config::Config>,
    sender: Sender,
    running_jobs: HashMap<job::Id, job::Job>,
    suspend: Suspend,
}

impl Model {
    pub(crate) fn new(config: Arc<config::Config>, sender: Sender, suspend: Suspend) -> Self {
        Model {
            config,
            sender,
            running_jobs: HashMap::new(),
            suspend,
        }
    }

    pub(crate) fn handle_event(&mut self, event: Event) -> eyre::Result<HandleEventOutcome> {
        match event {
            Event::JobStatusChange(status_change) => {
                match status_change.new_status {
                    job::Status::Started => self
                        .running_jobs
                        .insert(status_change.job.id, status_change.job),
                    job::Status::FinishedSuccessfully
                    | job::Status::FinishedWithError
                    | job::Status::Cancelled => self.running_jobs.remove(&status_change.job.id),
                };
                Ok(HandleEventOutcome::UpdateView)
            }
            Event::Suspend(suspend) => {
                self.suspend = suspend;
                Ok(HandleEventOutcome::UpdateView)
            }
            Event::ConfigReload(config_reload) => {
                self.config = config_reload.new_config;
                Ok(HandleEventOutcome::UpdateView)
            }

            Event::ToggleSuspended => {
                self.sender.send(self.suspend.toggle());
                Ok(HandleEventOutcome::Unchanged)
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
                self.sender.send(RequestShutdown);
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
        self.sender.send(job);
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

    pub(crate) fn status(&self) -> Status {
        if self.suspend.is_suspended() {
            Status::Suspended
        } else if !self.running_jobs.is_empty() {
            Status::Running
        } else {
            Status::Idle
        }
    }

    pub(crate) fn app_name(&self) -> &'static str {
        "Cirrus"
    }

    pub(crate) fn status_text(&self) -> Cow<'static, str> {
        if self.suspend.is_suspended() {
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

    pub(crate) fn tooltip(&self) -> String {
        format!("{} — {}", self.app_name(), self.status_text())
    }

    pub(crate) fn backups(&self) -> impl Iterator<Item = &config::backup::Name> + '_ {
        self.config.backups.iter().map(|(name, _)| name)
    }

    pub(crate) fn can_open_config_file(&self) -> bool {
        self.config.source.is_some()
    }

    pub(crate) fn is_suspended(&self) -> bool {
        self.suspend.is_suspended()
    }
}
