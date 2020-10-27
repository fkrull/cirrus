use cirrus_daemon::job;
use std::{borrow::Cow, collections::HashMap};
use winit::{
    event::Event,
    event_loop::{ControlFlow, EventLoop, EventLoopProxy},
};

const ICON: &[u8] = include_bytes!("../resources/icon.ico");

#[derive(Debug)]
pub(crate) struct StatusIcon {
    evloop_proxy: Option<EventLoopProxy<Events>>,
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
            let mut model = Model::new();
            let mut view = View::new(&evloop, &model).unwrap();
            send.send(evloop.create_proxy()).unwrap();
            evloop.run(move |event, _, control_flow| {
                *control_flow = ControlFlow::Wait;
                if let ViewUpdate::Changed = model.handle_event(event) {
                    view.update(&model).unwrap()
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
            .send_event(Events::JobStarted(job.clone()))?;
        Ok(())
    }

    pub(crate) fn job_succeeded(&mut self, job: &job::Job) -> eyre::Result<()> {
        self.evloop_proxy
            .as_ref()
            .unwrap()
            .send_event(Events::JobSucceeded(job.clone()))?;
        Ok(())
    }

    pub(crate) fn job_failed(&mut self, job: &job::Job) -> eyre::Result<()> {
        self.evloop_proxy
            .as_ref()
            .unwrap()
            .send_event(Events::JobFailed(job.clone()))?;
        Ok(())
    }
}

#[derive(Debug, PartialEq, Clone)]
enum Events {
    JobStarted(job::Job),
    JobSucceeded(job::Job),
    JobFailed(job::Job),
    Exit,
}

#[derive(Debug)]
enum ViewUpdate {
    Changed,
    Unchanged,
}

#[derive(Debug)]
struct Model {
    running_jobs: HashMap<job::Id, job::Job>,
}

impl Model {
    fn new() -> Self {
        Model {
            running_jobs: HashMap::new(),
        }
    }

    fn handle_event(&mut self, event: Event<Events>) -> ViewUpdate {
        match event {
            Event::UserEvent(Events::JobStarted(job)) => {
                self.running_jobs.insert(job.id, job);
                ViewUpdate::Changed
            }
            Event::UserEvent(Events::JobSucceeded(job)) => {
                self.running_jobs.remove(&job.id);
                ViewUpdate::Changed
            }
            Event::UserEvent(Events::JobFailed(job)) => {
                self.running_jobs.remove(&job.id);
                ViewUpdate::Changed
            }
            Event::UserEvent(Events::Exit) => {
                std::process::exit(0);
            }
            _ => ViewUpdate::Unchanged,
        }
    }

    fn tooltip(&self) -> Cow<'static, str> {
        if self.running_jobs.is_empty() {
            "Cirrus — idle".into()
        } else if self.running_jobs.len() == 1 {
            let job = self.running_jobs.values().next().unwrap();
            match &job.spec {
                job::Spec::Backup(_) => {
                    format!("Cirrus — backing up '{}'", &job.spec.name()).into()
                }
            }
        } else {
            format!("Cirrus — running {} jobs", self.running_jobs.len()).into()
        }
    }
}

struct View {
    tray_icon: trayicon::TrayIcon<Events>,
}

impl std::fmt::Debug for View {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("View")
            .field("tray_icon", &"<trayicon::TrayIcon>")
            .finish()
    }
}

impl View {
    fn new(evloop: &EventLoop<Events>, model: &Model) -> eyre::Result<Self> {
        let tray_icon = trayicon::TrayIconBuilder::new()
            .sender_winit(evloop.create_proxy())
            .tooltip(&model.tooltip())
            .icon_from_buffer(ICON)
            .menu(trayicon::MenuBuilder::new().item("Exit", Events::Exit))
            .build()
            .map_err(|e| eyre::eyre!("failed to create tray icon: {:?}", e))?;
        Ok(View { tray_icon })
    }

    fn update(&mut self, model: &Model) -> eyre::Result<()> {
        self.tray_icon
            .set_tooltip(&model.tooltip())
            .map_err(|e| eyre::eyre!("failed to set tooltip: {:?}", e))
    }
}
