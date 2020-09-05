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
pub struct BackupViewModel {
    base: BaseViewModel,
    name: String,
}

#[get("/backup/<name>")]
pub async fn backup(name: String, daemon: State<'_, Daemon>) -> TemplateResult {
    let name = backup::Name(name);
    let definition = daemon.config.backups.get(&name).ok_or(Error::NotFound)?;
    Template::render(
        "backup.html",
        BackupViewModel {
            base: base(&daemon)?,
            name: name.0,
        },
    )
}
