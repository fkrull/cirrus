use cirrus_daemon::job;
use std::{borrow::Cow, collections::HashMap};

#[derive(Debug, PartialEq, Clone)]
pub(super) enum Event {
    JobStarted(job::Job),
    JobSucceeded(job::Job),
    JobFailed(job::Job),
    Exit,
}

#[derive(Debug)]
pub(super) enum HandleEventOutcome {
    UpdateView,
    #[allow(dead_code)]
    Unchanged,
}

#[derive(Debug)]
pub(super) struct Model {
    running_jobs: HashMap<job::Id, job::Job>,
}

impl Model {
    pub(super) fn new() -> Self {
        Model {
            running_jobs: HashMap::new(),
        }
    }

    pub(super) fn handle_event(&mut self, event: Event) -> HandleEventOutcome {
        match event {
            Event::JobStarted(job) => {
                self.running_jobs.insert(job.id, job);
                HandleEventOutcome::UpdateView
            }
            Event::JobSucceeded(job) => {
                self.running_jobs.remove(&job.id);
                HandleEventOutcome::UpdateView
            }
            Event::JobFailed(job) => {
                self.running_jobs.remove(&job.id);
                HandleEventOutcome::UpdateView
            }
            Event::Exit => {
                std::process::exit(0);
            }
        }
    }

    fn app_name(&self) -> &'static str {
        "Cirrus"
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
}
