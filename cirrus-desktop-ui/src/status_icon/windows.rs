use std::sync::mpsc::Sender;
use winit::event_loop::{ControlFlow, EventLoop, EventLoopBuilder, EventLoopProxy};

fn tray_icon_thread(
    mut model: super::Model,
    evloop_proxy_send: Sender<EventLoopProxy<super::Event>>,
) {
    use winit::platform::windows::EventLoopBuilderExtWindows;

    let evloop = EventLoopBuilder::with_user_event()
        .with_any_thread(true)
        .build();
    let mut view = View::new(&evloop, &model).unwrap();
    evloop_proxy_send.send(evloop.create_proxy()).unwrap();
    evloop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        if let winit::event::Event::UserEvent(event) = event {
            let outcome = model.handle_event(event).unwrap();
            if let super::HandleEventOutcome::UpdateView = outcome {
                view.update(&model).unwrap()
            }
        }
    });
}

#[derive(Debug)]
pub(crate) struct Handle {
    evloop_proxy: EventLoopProxy<super::Event>,
}

impl Handle {
    pub(crate) fn check() -> eyre::Result<()> {
        Ok(())
    }

    pub(crate) fn start(model: super::Model) -> eyre::Result<Self> {
        let (send, recv) = std::sync::mpsc::channel();
        std::thread::spawn(move || tray_icon_thread(model, send));
        let evloop_proxy = recv.recv()?;
        Ok(Handle { evloop_proxy })
    }

    pub(crate) fn send(&mut self, event: super::Event) -> eyre::Result<()> {
        self.evloop_proxy.send_event(event)?;
        Ok(())
    }
}

struct View {
    tray_icon: trayicon::TrayIcon<super::Event>,
}

impl View {
    fn new(evloop: &EventLoop<super::Event>, model: &super::Model) -> eyre::Result<Self> {
        let evloop_proxy = evloop.create_proxy();
        let tray_icon = trayicon::TrayIconBuilder::<super::Event>::new()
            .sender_callback(move |ev| evloop_proxy.send_event(ev.clone()).unwrap())
            .tooltip(&model.tooltip())
            .icon(icon_for_status(model)?.clone())
            .menu(menu(model))
            .build()
            .map_err(|e| eyre::eyre!("failed to create tray icon: {:?}", e))?;
        Ok(View { tray_icon })
    }

    fn update(&mut self, model: &super::Model) -> eyre::Result<()> {
        self.tray_icon
            .set_tooltip(&model.tooltip())
            .map_err(|e| eyre::eyre!("failed to set tooltip: {:?}", e))?;
        self.tray_icon
            .set_icon(icon_for_status(model)?)
            .map_err(|e| eyre::eyre!("failed to set icon: {:?}", e))?;
        self.tray_icon
            .set_menu(&menu(model))
            .map_err(|e| eyre::eyre!("failed to set menu: {:?}", e))?;
        Ok(())
    }
}

fn menu(model: &super::Model) -> trayicon::MenuBuilder<super::Event> {
    let backups_menu = model
        .backups()
        .fold(trayicon::MenuBuilder::new(), |menu, name| {
            menu.item(&name.0, super::Event::RunBackup(name.clone()))
        });
    trayicon::MenuBuilder::new()
        .submenu("Run Backup", backups_menu)
        .checkable(
            "Suspended",
            model.is_suspended(),
            super::Event::ToggleSuspended,
        )
        .separator()
        .with(trayicon::MenuItem::Item {
            name: "Open Configuration".to_owned(),
            id: super::Event::OpenConfigFile,
            disabled: !model.can_open_config_file(),
            icon: None,
        })
        .item("Exit", super::Event::Exit)
}

fn icon_for_status(model: &super::Model) -> eyre::Result<&'static trayicon::Icon> {
    match model.status() {
        super::Status::Idle => icons::idle(),
        super::Status::Running => icons::running(),
        super::Status::Suspended => icons::suspend(),
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
    static SUSPEND_LIGHT: Lazy<trayicon::Icon> =
        Lazy::new(|| load_icon(include_bytes!("../resources/cirrus-suspend.light.ico")).unwrap());
    static SUSPEND_DARK: Lazy<trayicon::Icon> =
        Lazy::new(|| load_icon(include_bytes!("../resources/cirrus-suspend.dark.ico")).unwrap());

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

    pub(super) fn suspend() -> eyre::Result<&'static trayicon::Icon> {
        match systray_theme()? {
            SystrayTheme::Light => Ok(&SUSPEND_DARK),
            SystrayTheme::Dark => Ok(&SUSPEND_LIGHT),
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
