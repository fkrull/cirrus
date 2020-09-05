use crate::{
    assets::templates::{Template, TemplateResult},
    base::{base, BaseViewModel},
    Error,
};
use cirrus_core::model::{backup, repo};
use cirrus_daemon::Daemon;
use rocket::{get, State};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct RepoViewModel {
    base: BaseViewModel,
    name: String,
}

#[get("/repo/<name>")]
pub async fn repo(name: String, daemon: State<'_, Daemon>) -> TemplateResult {
    let name = repo::Name(name);
    let definition = daemon
        .config
        .repositories
        .get(&name)
        .ok_or(Error::NotFound)?;
    Template::render(
        "repo.html",
        RepoViewModel {
            base: base(&daemon)?,
            name: name.0,
        },
    )
}
