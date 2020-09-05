use super::content_type;
use crate::ServerError;
use anyhow::anyhow;
use log::error;
use rocket::{
    fairing::{Fairing, Info, Kind},
    http::Status,
    response::{Content, Responder},
    Request, Rocket,
};
use serde::Serialize;
use std::{borrow::Cow, path::Path};
use tera::{Context, Tera};

#[derive(Serialize)]
pub struct NoContext {}

#[derive(Debug)]
pub struct Template {
    name: Cow<'static, str>,
    context: Context,
}

pub type TemplateResult = Result<Template, ServerError>;

impl Template {
    pub fn render(name: impl Into<Cow<'static, str>>, context: impl Serialize) -> TemplateResult {
        let name = name.into();
        let context = Context::from_serialize(context)
            .map_err(|e| anyhow!("failed to serialize template context: {}", e))?;
        Ok(Template { name, context })
    }

    pub fn fairing() -> impl Fairing {
        TemplateFairing
    }
}

impl<'r> Responder<'r, 'static> for Template {
    fn respond_to(self, req: &'r Request<'_>) -> rocket::response::Result<'static> {
        let tera = req
            .managed_state::<templates_impl::TeraState>()
            .ok_or_else(|| {
                error!("template state is missing from app");
                Status::InternalServerError
            })?;
        let content_type = content_type(Path::new(&self.name.as_ref()));
        let render = tera.render(&self.name, &self.context).map_err(|err| {
            error!("failed to render template: {}", err);
            Status::InternalServerError
        })?;
        Content(content_type, render).respond_to(req)
    }
}

#[derive(Debug)]
struct TemplateFairing;

#[cfg(not(feature = "bundled-assets"))]
mod templates_impl {
    use super::*;
    use rocket::Data;
    use std::{path::Path, sync::RwLock};

    #[derive(Debug)]
    pub(super) struct TeraState(RwLock<Tera>);

    impl TeraState {
        fn reload(&self) {
            if let Err(err) = self.0.write().unwrap().full_reload() {
                error!("template reload failed: {}", err);
            }
        }

        pub(super) fn render(&self, name: &str, context: &Context) -> anyhow::Result<String> {
            self.0
                .read()
                .unwrap()
                .render(name, context)
                .map_err(|e| e.into())
        }
    }

    #[rocket::async_trait]
    impl Fairing for TemplateFairing {
        fn info(&self) -> Info {
            Info {
                name: "Templates",
                kind: Kind::Attach | Kind::Request,
            }
        }

        async fn on_attach(&self, rocket: Rocket) -> Result<Rocket, Rocket> {
            let tera = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("templates")
                .join("**")
                .to_str()
                .and_then(|path| match Tera::new(path) {
                    Ok(tera) => Some(tera),
                    Err(err) => {
                        error!("failed to create template context: {}", err);
                        None
                    }
                });
            match tera {
                Some(tera) => Ok(rocket.manage(TeraState(RwLock::new(tera)))),
                None => Err(rocket),
            }
        }

        async fn on_request(&self, req: &mut Request<'_>, _data: &Data) {
            if let Some(tera) = req.managed_state::<TeraState>() {
                tera.reload();
            }
        }
    }
}

#[cfg(feature = "bundled-assets")]
mod templates_impl {
    use super::*;
    use include_dir::{include_dir, Dir, DirEntry};

    #[derive(Debug)]
    pub(super) struct TeraState(Tera);

    impl TeraState {
        pub(super) fn render(&self, name: &str, context: &Context) -> anyhow::Result<String> {
            self.0.render(name, context).map_err(|e| e.into())
        }
    }

    const TEMPLATE_FILES: Dir = include_dir!("templates");

    #[rocket::async_trait]
    impl Fairing for TemplateFairing {
        fn info(&self) -> Info {
            Info {
                name: "Templates",
                kind: Kind::Attach,
            }
        }

        async fn on_attach(&self, rocket: Rocket) -> Result<Rocket, Rocket> {
            let bundled_templates =
                TEMPLATE_FILES
                    .find("**")
                    .unwrap()
                    .filter_map(|entry| match entry {
                        DirEntry::File(file) => {
                            Some((file.path().to_str().unwrap(), file.contents_utf8().unwrap()))
                        }
                        DirEntry::Dir(_) => None,
                    });

            let mut tera = Tera::default();
            match tera.add_raw_templates(bundled_templates) {
                Ok(()) => Ok(rocket.manage(TeraState(tera))),
                Err(err) => {
                    error!("failed to create template context: {}", err);
                    Err(rocket)
                }
            }
        }
    }
}
