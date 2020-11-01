use super::model;
use cirrus_daemon::job;
use winit::{
    event::Event,
    event_loop::{ControlFlow, EventLoop, EventLoopProxy},
};

const ICON: &[u8] = include_bytes!("../resources/icon.ico");

#[derive(Debug)]
pub(crate) struct StatusIcon {
    evloop_proxy: Option<EventLoopProxy<model::Event>>,
}

impl StatusIcon {
    pub(crate) fn new() -> eyre::Result<Self> {
        Ok(StatusIcon { evloop_proxy: None })
    }

    pub(crate) fn start(&mut self) -> eyre::Result<()> {
        use winit::platform::windows::EventLoopExtWindows;

        let (send, recv) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let evloop = EventLoop::new_any_thread();
            let mut model = model::Model::new();
            let mut view = View::new(&evloop, &model).unwrap();
            send.send(evloop.create_proxy()).unwrap();
            evloop.run(move |event, _, control_flow| {
                *control_flow = ControlFlow::Wait;
                if let Event::UserEvent(event) = event {
                    if let model::HandleEventOutcome::UpdateView = model.handle_event(event) {
                        view.update(&model).unwrap()
                    }
                }
            });
        });

        let evloop_proxy = recv.recv()?;
        self.evloop_proxy = Some(evloop_proxy);
        Ok(())
    }

    pub(crate) fn job_started(&mut self, job: &job::Job) -> eyre::Result<()> {
        self.evloop_proxy
            .as_ref()
            .unwrap()
            .send_event(model::Event::JobStarted(job.clone()))?;
        Ok(())
    }

    pub(crate) fn job_succeeded(&mut self, job: &job::Job) -> eyre::Result<()> {
        self.evloop_proxy
            .as_ref()
            .unwrap()
            .send_event(model::Event::JobSucceeded(job.clone()))?;
        Ok(())
    }

    pub(crate) fn job_failed(&mut self, job: &job::Job) -> eyre::Result<()> {
        self.evloop_proxy
            .as_ref()
            .unwrap()
            .send_event(model::Event::JobFailed(job.clone()))?;
        Ok(())
    }
}

struct View {
    tray_icon: trayicon::TrayIcon<model::Event>,
}

impl View {
    fn new(evloop: &EventLoop<model::Event>, model: &model::Model) -> eyre::Result<Self> {
        let tray_icon = trayicon::TrayIconBuilder::new()
            .sender_winit(evloop.create_proxy())
            .tooltip(&model.tooltip())
            .icon_from_buffer(ICON)
            .menu(trayicon::MenuBuilder::new().item("Exit", model::Event::Exit))
            .build()
            .map_err(|e| eyre::eyre!("failed to create tray icon: {:?}", e))?;
        Ok(View { tray_icon })
    }

    fn update(&mut self, model: &model::Model) -> eyre::Result<()> {
        self.tray_icon
            .set_tooltip(&model.tooltip())
            .map_err(|e| eyre::eyre!("failed to set tooltip: {:?}", e))
    }
}
