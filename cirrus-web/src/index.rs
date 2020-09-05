use crate::assets::templates::Template;
use crate::base::{base, BaseViewModel};
use crate::ServerError;
use cirrus_daemon::Daemon;
use rocket::get;
use rocket::State;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct IndexViewModel {
    pub base: BaseViewModel,
}

#[get("/")]
pub fn index(daemon: State<Daemon>) -> Result<Template, ServerError> {
    Template::render(
        "index.html",
        IndexViewModel {
            base: base(&daemon)?,
        },
    )
}
