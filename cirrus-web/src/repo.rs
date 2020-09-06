use crate::{
    assets::templates::{Template, TemplateResult},
    base::{base, BaseViewModel},
    Error,
};
use cirrus_core::model::repo;
use cirrus_daemon::Daemon;
use rocket::{get, uri, State};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct RepoViewModel {
    base: BaseViewModel,
    name: String,
    url: String,
    backups: Vec<BackupForRepo>,
}

#[derive(Debug, Serialize)]
pub struct BackupForRepo {
    pub name: String,
    pub link: String,
}

#[get("/repo/<name>")]
pub async fn repo(name: String, daemon: State<'_, Daemon>) -> TemplateResult {
    let name = repo::Name(name);
    let definition = daemon
        .config
        .repositories
        .get(&name)
        .ok_or(Error::NotFound)?;
    let backups = daemon
        .config
        .backups
        .0
        .iter()
        .filter(|(_, backup)| backup.repository == name)
        .map(|(name, _)| name.0.clone())
        .map(|name| {
            let link = uri!(crate::backup::backup: name = &name).to_string();
            BackupForRepo { name, link }
        })
        .collect();
    Template::render(
        "repo.html",
        RepoViewModel {
            base: base(&daemon).await?,
            name: name.0,
            url: definition.url.0.clone(),
            backups,
        },
    )
}
