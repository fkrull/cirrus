use super::model;
use cirrus_daemon::job;

const APP_ID: &'static str = "io.gitlab.fkrull.cirrus.Cirrus";

pub(crate) struct StatusIcon {
    handle: Option<ksni::Handle<model::Model>>,
}

impl std::fmt::Debug for StatusIcon {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StatusIcon")
            .field("handle", &"...")
            .finish()
    }
}

impl StatusIcon {
    pub(crate) fn new() -> eyre::Result<Self> {
        Ok(StatusIcon { handle: None })
    }

    pub(crate) fn start(&mut self) -> eyre::Result<()> {
        let service = ksni::TrayService::new(model::Model::new());
        self.handle = Some(service.handle());
        service.spawn();
        Ok(())
    }

    pub(crate) fn job_started(&mut self, job: &job::Job) -> eyre::Result<()> {
        self.handle.as_ref().unwrap().update(|model| {
            model.handle_event(model::Event::JobStarted(job.clone()));
        });
        Ok(())
    }

    pub(crate) fn job_succeeded(&mut self, job: &job::Job) -> eyre::Result<()> {
        self.handle.as_ref().unwrap().update(|model| {
            model.handle_event(model::Event::JobSucceeded(job.clone()));
        });
        Ok(())
    }

    pub(crate) fn job_failed(&mut self, job: &job::Job) -> eyre::Result<()> {
        self.handle.as_ref().unwrap().update(|model| {
            model.handle_event(model::Event::JobFailed(job.clone()));
        });
        Ok(())
    }
}

impl ksni::Tray for model::Model {
    fn id(&self) -> String {
        APP_ID.to_owned()
    }

    fn title(&self) -> String {
        self.app_name().to_owned()
    }

    fn tool_tip(&self) -> ksni::ToolTip {
        ksni::ToolTip {
            title: self.tooltip().to_owned(),
            ..Default::default()
        }
    }

    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        use ksni::menu::*;

        vec![StandardItem {
            label: "Exit".to_owned(),
            activate: Box::new(|this: &mut model::Model| {
                this.handle_event(model::Event::Exit);
            }),
            ..Default::default()
        }
        .into()]
    }
}
