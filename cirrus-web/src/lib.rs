use crate::assets::{static_files::StaticFiles, templates::Template};
use cirrus_daemon::Daemon;
use log::error;
use rocket::{http::Status, response::Responder, routes, Request, Response};

mod assets;
mod backup;
mod base;
mod index;
mod repo;

#[derive(Debug)]
pub struct ServerError(anyhow::Error);

impl From<anyhow::Error> for ServerError {
    fn from(error: anyhow::Error) -> Self {
        ServerError(error)
    }
}

impl<'r> Responder<'r, 'static> for ServerError {
    fn respond_to(self, _req: &'r Request<'_>) -> rocket::response::Result<'static> {
        error!("Internal server error: {}", self.0);
        Response::build().status(Status::InternalServerError).ok()
    }
}

pub async fn launch(daemon: Daemon) -> anyhow::Result<()> {
    rocket::ignite()
        .manage(daemon)
        .mount("/", routes![index::index, repo::repo, backup::backup])
        .mount("/static", StaticFiles)
        .attach(Template::fairing())
        .launch()
        .await?;
    Ok(())
}
