const APP_ID: &str = "io.gitlab.fkrull.cirrus.Cirrus";

pub(crate) struct Handle {
    handle: ksni::Handle<super::Model>,
}

impl std::fmt::Debug for Handle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StatusIcon")
            .field("handle", &"...")
            .finish()
    }
}

impl Handle {
    pub(crate) fn check() -> eyre::Result<()> {
        check_session_dbus_connection()
    }

    pub(crate) fn start(model: super::Model) -> eyre::Result<Self> {
        let service = ksni::TrayService::new(model);
        let handle = service.handle();
        service.spawn();
        Ok(Handle { handle })
    }

    pub(crate) fn send(&mut self, event: super::Event) -> eyre::Result<()> {
        self.handle.update(|model| {
            model.handle_event(event.clone()).unwrap();
        });
        Ok(())
    }
}

fn check_session_dbus_connection() -> eyre::Result<()> {
    zbus::blocking::Connection::session()?;
    Ok(())
}

impl ksni::Tray for super::Model {
    fn id(&self) -> String {
        APP_ID.to_owned()
    }

    fn title(&self) -> String {
        self.app_name().to_owned()
    }

    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        match self.status() {
            super::Status::Idle => icons::idle().clone(),
            super::Status::Running => icons::running().clone(),
            super::Status::Suspended => icons::suspend().clone(),
        }
    }

    fn tool_tip(&self) -> ksni::ToolTip {
        ksni::ToolTip {
            title: self.tooltip(),
            ..Default::default()
        }
    }

    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        use ksni::menu::*;

        let backups_menu = self
            .backups()
            .map(|name| {
                let name = name.clone();
                StandardItem {
                    label: name.0.clone(),
                    activate: Box::new(move |this: &mut super::Model| {
                        this.handle_event(super::Event::RunBackup(name.clone()))
                            .unwrap();
                    }),
                    ..Default::default()
                }
                .into()
            })
            .collect();

        vec![
            StandardItem {
                label: self.status_text().into_owned(),
                enabled: false,
                ..Default::default()
            }
            .into(),
            SubMenu {
                label: "Run Backup".to_string(),
                submenu: backups_menu,
                ..Default::default()
            }
            .into(),
            CheckmarkItem {
                label: "Suspended".to_owned(),
                checked: self.is_suspended(),
                activate: Box::new(move |this: &mut super::Model| {
                    this.handle_event(super::Event::ToggleSuspended).unwrap();
                }),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: "Open Configuration".to_owned(),
                activate: Box::new(move |this: &mut super::Model| {
                    this.handle_event(super::Event::OpenConfigFile).unwrap();
                }),
                enabled: self.can_open_config_file(),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: "Exit".to_owned(),
                activate: Box::new(move |this: &mut super::Model| {
                    this.handle_event(super::Event::Exit).unwrap();
                }),
                ..Default::default()
            }
            .into(),
        ]
    }
}

mod icons {
    use once_cell::sync::Lazy;

    static IDLE_LIGHT: Lazy<Vec<ksni::Icon>> = Lazy::new(|| {
        const ICON_DATA: [&[u8]; 4] = [
            include_bytes!("../resources/16/cirrus-idle.light.png"),
            include_bytes!("../resources/24/cirrus-idle.light.png"),
            include_bytes!("../resources/32/cirrus-idle.light.png"),
            include_bytes!("../resources/48/cirrus-idle.light.png"),
        ];
        ICON_DATA
            .iter()
            .map(|&data| load_png(data))
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
    });
    static RUNNING_LIGHT: Lazy<Vec<ksni::Icon>> = Lazy::new(|| {
        const ICON_DATA: [&[u8]; 4] = [
            include_bytes!("../resources/16/cirrus-running.light.png"),
            include_bytes!("../resources/24/cirrus-running.light.png"),
            include_bytes!("../resources/32/cirrus-running.light.png"),
            include_bytes!("../resources/48/cirrus-running.light.png"),
        ];
        ICON_DATA
            .iter()
            .map(|&data| load_png(data))
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
    });

    static SUSPEND_LIGHT: Lazy<Vec<ksni::Icon>> = Lazy::new(|| {
        const ICON_DATA: [&[u8]; 4] = [
            include_bytes!("../resources/16/cirrus-suspend.light.png"),
            include_bytes!("../resources/24/cirrus-suspend.light.png"),
            include_bytes!("../resources/32/cirrus-suspend.light.png"),
            include_bytes!("../resources/48/cirrus-suspend.light.png"),
        ];
        ICON_DATA
            .iter()
            .map(|&data| load_png(data))
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
    });

    pub(super) fn idle() -> &'static Vec<ksni::Icon> {
        &IDLE_LIGHT
    }

    pub(super) fn running() -> &'static Vec<ksni::Icon> {
        &RUNNING_LIGHT
    }

    pub(super) fn suspend() -> &'static Vec<ksni::Icon> {
        &SUSPEND_LIGHT
    }

    fn load_png(data: &[u8]) -> eyre::Result<ksni::Icon> {
        use png::{BitDepth, ColorType, Decoder};

        let mut reader = Decoder::new(data).read_info()?;
        let info = reader.info();
        if info.bit_depth != BitDepth::Eight {
            return Err(eyre::eyre!(
                "unsupported PNG bit depth: {:?}",
                info.bit_depth
            ));
        }
        if info.color_type != ColorType::Rgba {
            return Err(eyre::eyre!("unsupported PNG format: {:?}", info.color_type));
        }

        let mut data = vec![0u8; reader.output_buffer_size()];
        reader.next_frame(&mut data)?;
        let info = reader.info();
        rgba_to_argb(&mut data);

        Ok(ksni::Icon {
            width: info.width as i32,
            height: info.height as i32,
            data,
        })
    }

    fn rgba_to_argb(data: &mut [u8]) {
        for offset in (0..data.len()).step_by(4) {
            let alpha = data[offset + 3];
            data.copy_within(offset..offset + 3, offset + 1);
            data[offset] = alpha;
        }
    }
}
