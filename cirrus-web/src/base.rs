use cirrus_core::model::{backup, repo};
use cirrus_daemon::Daemon;
use rocket::uri;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct BaseViewModel {
    pub instance_name: String,
    pub nav: NavViewModel,
}

#[derive(Debug, Serialize)]
pub struct NavViewModel {
    pub repos: Vec<NavRepo>,
    pub backups: Vec<NavBackup>,
}

#[derive(Debug, Serialize)]
pub struct NavRepo {
    pub name: String,
    pub link: String,
}

#[derive(Debug, Serialize)]
pub struct NavBackup {
    pub name: String,
    pub link: String,
}

fn repo_uri(name: &repo::Name) -> String {
    uri!(crate::repo::repo: name = &name.0).to_string()
}

fn backup_uri(name: &backup::Name) -> String {
    uri!(crate::backup::backup: name = &name.0).to_string()
}

pub fn base(daemon: &Daemon) -> anyhow::Result<BaseViewModel> {
    let repos = daemon
        .config
        .repositories
        .0
        .iter()
        .map(|(name, definition)| NavRepo {
            name: name.0.clone(),
            link: repo_uri(name),
        })
        .collect::<Vec<_>>();
    let backups = daemon
        .config
        .backups
        .0
        .iter()
        .map(|(name, definition)| NavBackup {
            name: name.0.clone(),
            link: backup_uri(name),
        })
        .collect::<Vec<_>>();

    Ok(BaseViewModel {
        instance_name: daemon.instance_name.clone(),
        nav: NavViewModel { repos, backups },
    })
}
