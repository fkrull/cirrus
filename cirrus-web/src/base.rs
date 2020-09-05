use cirrus_daemon::Daemon;
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
pub enum BackupStatus {
    Idle,
    Running,
    Error,
}

#[derive(Debug, Serialize)]
pub struct NavBackup {
    pub name: String,
    pub link: String,
    pub status: BackupStatus,
}

pub fn base(daemon: &Daemon) -> anyhow::Result<BaseViewModel> {
    Ok(BaseViewModel {
        instance_name: daemon.instance_name.clone(),
        nav: NavViewModel {
            repos: vec![],
            backups: vec![],
        },
    })
}
