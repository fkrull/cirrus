use crate::{
    assets::templates::{Template, TemplateResult},
    base::{base, BaseViewModel},
    Error,
};
use cirrus_core::model::backup;
use cirrus_daemon::Daemon;
use rocket::{get, uri, State};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct BackupViewModel {
    base: BaseViewModel,
    name: String,
    path: String,
    excludes: Vec<String>,
    repo: RepoForBackup,
}

#[derive(Debug, Serialize)]
pub struct RepoForBackup {
    pub name: String,
    pub link: String,
}

#[get("/backup/<name>")]
pub async fn backup(name: String, daemon: State<'_, Daemon>) -> TemplateResult {
    let name = backup::Name(name);
    let definition = daemon.config.backups.get(&name).ok_or(Error::NotFound)?;
    let repo_name = definition.repository.0.clone();
    let repo_link = uri!(crate::repo::repo: name = &repo_name).to_string();
    Template::render(
        "backup.html",
        BackupViewModel {
            base: base(&daemon)?,
            name: name.0,
            path: definition.path.0.clone(),
            excludes: definition.excludes.iter().map(|x| x.0.clone()).collect(),
            repo: RepoForBackup {
                name: repo_name,
                link: repo_link,
            },
        },
    )
}
