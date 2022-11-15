use snisni::{menu, menubuilder::MenuBuilder, sni};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

const APP_ID: &str = "io.gitlab.fkrull.cirrus.Cirrus";

pub(crate) async fn start(model: super::Model) -> eyre::Result<Handle> {
    let (mut status_notifier_item, send) = StatusNotifierItem::new(model).await?;
    tokio::spawn(async move {
        if let Err(error) = status_notifier_item.run().await {
            tracing::warn!(%error, "error while running the status icon");
        }
    });
    Ok(Handle(send))
}

#[derive(Debug)]
pub(crate) struct Handle(UnboundedSender<super::Event>);

impl Handle {
    pub(crate) fn send(&mut self, event: super::Event) -> eyre::Result<()> {
        self.0.send(event)?;
        Ok(())
    }
}

fn sni_model(model: &super::Model) -> sni::Model {
    let icon = match model.status() {
        super::Status::Idle => icons::idle(),
        super::Status::Running => icons::running(),
        super::Status::Suspended => icons::suspend(),
    };
    sni::Model {
        icon: icon.clone(),
        id: APP_ID.to_string(),
        title: model.app_name().to_string(),
        tooltip: sni::Tooltip {
            title: model.tooltip(),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn menu(model: &super::Model) -> menu::Model<super::Event> {
    let items = MenuBuilder::new_root()
        .disabled(model.status_text())
        .sub_menu(
            MenuBuilder::new("Run Backup").items(model.backups().map(|name| menu::Item {
                label: name.0.clone(),
                message: Some(super::Event::RunBackup(name.clone())),
                ..Default::default()
            })),
        )
        .item(menu::Item {
            label: "Suspended".to_string(),
            r#type: menu::Type::Checkmark {
                selected: model.is_suspended(),
            },
            message: Some(super::Event::ToggleSuspended),
            ..Default::default()
        })
        .separator()
        .item(menu::Item {
            label: "Open Configuration".to_string(),
            message: Some(super::Event::OpenConfigFile),
            enabled: model.can_open_config_file(),
            ..Default::default()
        })
        .standard_item("Exit", super::Event::Exit)
        .build();

    menu::Model {
        items,
        ..Default::default()
    }
}

#[derive(Debug)]
struct StatusNotifierItem {
    model: super::Model,
    handle: snisni::Handle<super::Event>,
    recv: UnboundedReceiver<super::Event>,
}

impl StatusNotifierItem {
    async fn new(
        model: super::Model,
    ) -> eyre::Result<(StatusNotifierItem, UnboundedSender<super::Event>)> {
        let (send, recv) = tokio::sync::mpsc::unbounded_channel();
        let send2 = send.clone();
        let handle = snisni::Handle::new(
            snisni::SniName::new(0),
            sni_model(&model),
            menu(&model),
            Box::new(snisni::DiscardEvents),
            Box::new(move |ev: menu::Event<super::Event>| {
                let send2 = send2.clone();
                async move {
                    if ev.r#type == menu::EventType::Clicked {
                        send2.send(ev.message).unwrap();
                    }
                }
            }),
        )
        .await?;
        let sni = StatusNotifierItem {
            model,
            handle,
            recv,
        };
        Ok((sni, send))
    }

    #[tracing::instrument(name = "StatusNotifierItem", skip_all)]
    async fn run(&mut self) -> eyre::Result<()> {
        self.handle.register().await?;
        while let Some(ev) = self.recv.recv().await {
            if let super::HandleEventOutcome::UpdateView = self.model.handle_event(ev)? {
                self.handle.update(|m| *m = sni_model(&self.model)).await?;
                self.handle.update_menu(|m| *m = menu(&self.model)).await?;
            }
        }
        Ok(())
    }
}

mod icons {
    use once_cell::sync::Lazy;
    use snisni::sni;

    static IDLE_LIGHT: Lazy<sni::Icon> = Lazy::new(|| {
        const ICON_DATA: [&[u8]; 4] = [
            include_bytes!("resources/16/cirrus-idle.light.png"),
            include_bytes!("resources/24/cirrus-idle.light.png"),
            include_bytes!("resources/32/cirrus-idle.light.png"),
            include_bytes!("resources/48/cirrus-idle.light.png"),
        ];
        to_icon(&ICON_DATA)
    });
    static RUNNING_LIGHT: Lazy<sni::Icon> = Lazy::new(|| {
        const ICON_DATA: [&[u8]; 4] = [
            include_bytes!("resources/16/cirrus-running.light.png"),
            include_bytes!("resources/24/cirrus-running.light.png"),
            include_bytes!("resources/32/cirrus-running.light.png"),
            include_bytes!("resources/48/cirrus-running.light.png"),
        ];
        to_icon(&ICON_DATA)
    });

    static SUSPEND_LIGHT: Lazy<sni::Icon> = Lazy::new(|| {
        const ICON_DATA: [&[u8]; 4] = [
            include_bytes!("resources/16/cirrus-suspend.light.png"),
            include_bytes!("resources/24/cirrus-suspend.light.png"),
            include_bytes!("resources/32/cirrus-suspend.light.png"),
            include_bytes!("resources/48/cirrus-suspend.light.png"),
        ];
        to_icon(&ICON_DATA)
    });

    pub(crate) fn idle() -> &'static sni::Icon {
        &IDLE_LIGHT
    }

    pub(crate) fn running() -> &'static sni::Icon {
        &RUNNING_LIGHT
    }

    pub(crate) fn suspend() -> &'static sni::Icon {
        &SUSPEND_LIGHT
    }

    fn to_icon(icon_data: &[&[u8]]) -> sni::Icon {
        let pixmaps = icon_data
            .iter()
            .map(|&data| load_png(data))
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        sni::Icon {
            name: String::new(),
            pixmaps,
        }
    }

    fn load_png(data: &[u8]) -> eyre::Result<sni::Pixmap> {
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

        Ok(sni::Pixmap {
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
