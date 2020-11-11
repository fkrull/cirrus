use super::model;
use cirrus_daemon::job;

const APP_ID: &str = "io.gitlab.fkrull.cirrus.Cirrus";

pub(crate) struct StatusIcon {
    deps: crate::Deps,
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
    pub(crate) fn new(deps: crate::Deps) -> eyre::Result<Self> {
        Ok(StatusIcon { deps, handle: None })
    }

    pub(crate) fn start(&mut self) -> eyre::Result<()> {
        let model = model::Model::new(self.deps.clone());
        let service = ksni::TrayService::new(model);
        self.handle = Some(service.handle());
        service.spawn();
        Ok(())
    }

    pub(crate) fn job_started(&mut self, job: &job::Job) -> eyre::Result<()> {
        self.handle.as_ref().unwrap().update(|model| {
            model
                .handle_event(model::Event::JobStarted(job.clone()))
                .unwrap();
        });
        Ok(())
    }

    pub(crate) fn job_succeeded(&mut self, job: &job::Job) -> eyre::Result<()> {
        self.handle.as_ref().unwrap().update(|model| {
            model
                .handle_event(model::Event::JobSucceeded(job.clone()))
                .unwrap();
        });
        Ok(())
    }

    pub(crate) fn job_failed(&mut self, job: &job::Job) -> eyre::Result<()> {
        self.handle.as_ref().unwrap().update(|model| {
            model
                .handle_event(model::Event::JobFailed(job.clone()))
                .unwrap();
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

    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        match self.status() {
            model::Status::Idle => icons::idle().clone(),
            model::Status::Running => icons::running().clone(),
        }
    }

    fn tool_tip(&self) -> ksni::ToolTip {
        ksni::ToolTip {
            title: self.tooltip().to_owned(),
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
                    activate: Box::new(move |this: &mut model::Model| {
                        this.handle_event(model::Event::RunBackup(name.clone()))
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
            MenuItem::Sepatator,
            StandardItem {
                label: "Exit".to_owned(),
                activate: Box::new(|this: &mut model::Model| {
                    this.handle_event(model::Event::Exit).unwrap();
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

    pub(super) fn idle() -> &'static Vec<ksni::Icon> {
        &IDLE_LIGHT
    }

    pub(super) fn running() -> &'static Vec<ksni::Icon> {
        &RUNNING_LIGHT
    }

    fn load_png(data: &[u8]) -> eyre::Result<ksni::Icon> {
        use png::{BitDepth, ColorType, Decoder};

        let (info, mut reader) = Decoder::new(data).read_info()?;
        if info.bit_depth != BitDepth::Eight {
            return Err(eyre::eyre!(
                "unsupported PNG bit depth: {:?}",
                info.bit_depth
            ));
        }
        if info.color_type != ColorType::RGBA {
            return Err(eyre::eyre!("unsupported PNG format: {:?}", info.color_type));
        }

        let mut data = vec![0u8; info.buffer_size()];
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
