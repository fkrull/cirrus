use super::model;
use cirrus_daemon::job;
use winit::{
    event::Event,
    event_loop::{ControlFlow, EventLoop, EventLoopProxy},
};

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
            .icon(icon_for_status(model)?.clone())
            .menu(trayicon::MenuBuilder::new().item("Exit", model::Event::Exit))
            .build()
            .map_err(|e| eyre::eyre!("failed to create tray icon: {:?}", e))?;
        Ok(View { tray_icon })
    }

    fn update(&mut self, model: &model::Model) -> eyre::Result<()> {
        self.tray_icon
            .set_tooltip(&model.tooltip())
            .map_err(|e| eyre::eyre!("failed to set tooltip: {:?}", e))?;
        self.tray_icon
            .set_icon(icon_for_status(model)?)
            .map_err(|e| eyre::eyre!("failed to set icon: {:?}", e))?;
        Ok(())
    }
}

fn icon_for_status(model: &model::Model) -> eyre::Result<&'static trayicon::Icon> {
    match model.status() {
        model::Status::Idle => icons::idle(),
        model::Status::Running => icons::running(),
    }
}

mod icons {
    use once_cell::sync::Lazy;

    static IDLE_LIGHT: Lazy<trayicon::Icon> =
        Lazy::new(|| load_icon(include_bytes!("../resources/cirrus-idle.light.ico")).unwrap());
    static IDLE_DARK: Lazy<trayicon::Icon> =
        Lazy::new(|| load_icon(include_bytes!("../resources/cirrus-idle.dark.ico")).unwrap());
    static RUNNING_LIGHT: Lazy<trayicon::Icon> =
        Lazy::new(|| load_icon(include_bytes!("../resources/cirrus-running.light.ico")).unwrap());
    static RUNNING_DARK: Lazy<trayicon::Icon> =
        Lazy::new(|| load_icon(include_bytes!("../resources/cirrus-running.dark.ico")).unwrap());

    pub(super) fn idle() -> eyre::Result<&'static trayicon::Icon> {
        match systray_theme()? {
            SystrayTheme::Light => Ok(&IDLE_DARK),
            SystrayTheme::Dark => Ok(&IDLE_LIGHT),
        }
    }

    pub(super) fn running() -> eyre::Result<&'static trayicon::Icon> {
        match systray_theme()? {
            SystrayTheme::Light => Ok(&RUNNING_DARK),
            SystrayTheme::Dark => Ok(&RUNNING_LIGHT),
        }
    }

    #[derive(Debug)]
    enum SystrayTheme {
        Light,
        Dark,
    }

    fn systray_theme() -> eyre::Result<SystrayTheme> {
        use winreg::enums::HKEY_CURRENT_USER;
        use winreg::RegKey;
        let personalize = RegKey::predef(HKEY_CURRENT_USER)
            .open_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize")?;
        let is_light: u32 = personalize.get_value("SystemUsesLightTheme")?;
        if is_light != 0 {
            Ok(SystrayTheme::Light)
        } else {
            Ok(SystrayTheme::Dark)
        }
    }

    fn load_icon(buffer: &'static [u8]) -> eyre::Result<trayicon::Icon> {
        trayicon::Icon::from_buffer(buffer, None, None)
            .map_err(|e| eyre::eyre!("failed to load icon: {:?}", e))
    }
}