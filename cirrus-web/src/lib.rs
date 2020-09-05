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
pub enum Error {
    NotFound,
    ServerError(anyhow::Error),
}

impl From<anyhow::Error> for Error {
    fn from(error: anyhow::Error) -> Self {
        Error::ServerError(error)
    }
}

impl<'r> Responder<'r, 'static> for Error {
    fn respond_to(self, _req: &'r Request<'_>) -> rocket::response::Result<'static> {
        match self {
            Error::NotFound => Response::build().status(Status::NotFound).ok(),
            Error::ServerError(error) => {
                error!("Internal server error: {}", error);
                Response::build().status(Status::InternalServerError).ok()
            }
        }
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
