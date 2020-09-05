use crate::{
    assets::templates::{Template, TemplateResult},
    base::{base, BaseViewModel},
};
use cirrus_daemon::Daemon;
use rocket::get;
use rocket::State;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct IndexViewModel {
    pub base: BaseViewModel,
}

#[get("/")]
pub fn index(daemon: State<Daemon>) -> TemplateResult {
    Template::render(
        "index.html",
        IndexViewModel {
            base: base(&daemon)?,
        },
    )
}
