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

        vec![
            StandardItem {
                label: self.status_text().into_owned(),
                enabled: false,
                ..Default::default()
            }
            .into(),
            MenuItem::Sepatator,
            StandardItem {
                label: "Exit".to_owned(),
                activate: Box::new(|this: &mut model::Model| {
                    this.handle_event(model::Event::Exit);
                }),
                ..Default::default()
            }
            .into(),
        ]
    }
}

fn load_png(data: &[u8]) -> eyre::Result<ksni::Icon> {
    use png::{BitDepth, ColorType, Decoder, Transformations};

    let mut decoder = Decoder::new(data);
    decoder.set_transformations(Transformations::SWAP_ALPHA);
    let (info, mut reader) = decoder.read_info()?;
    if info.bit_depth != BitDepth::Eight {
        return Err(eyre::eyre!(
            "unsupported PNG bit depth: {:?}",
            info.bit_depth
        ));
    }
    if info.color_type != ColorType::RGBA {
        return Err(eyre::eyre!("unsupported PNG format: {:?}", info.color_type));
    }

    let mut data =
        vec![0u8; info.width * info.height * info.color_type.samples() * info.bit_depth as u8];
    let mut pos = 0;
    while let Some(row) = reader.next_row()? {
        data[pos..pos + row.len()].copy_from_slice(row);
        pos += row.len();
    }
    Ok(ksni::Icon {
        width: info.width as i32,
        height: info.height as i32,
        data,
    })
}
