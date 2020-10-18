use cirrus_core::model::backup;
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
    pub overview: NavItem,
    pub repos: Vec<NavRepo>,
    pub backups: Vec<NavBackup>,
}

#[derive(Debug, Serialize)]
pub struct NavItem {
    pub name: String,
    pub link: String,
}

#[derive(Debug, Serialize)]
pub struct NavRepo {
    pub name: String,
    pub link: String,
}

#[derive(Debug, Serialize)]
#[serde(tag = "status")]
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

async fn nav_backup_item(daemon: &Daemon, name: &backup::Name) -> NavBackup {
    /*let job = daemon.jobs_repo.backup_jobs(name).await.pop();
    let status = match job {
        Some(job) if job.is_running() => BackupStatus::Running,
        Some(job) if job.is_finished() && job.is_error() => BackupStatus::Error,
        _ => BackupStatus::Idle,
    };
    NavBackup {
        name: name.0.clone(),
        link: uri!(crate::backup::backup: name = &name.0).to_string(),
        status,
    }*/
    unimplemented!()
}

pub async fn base(daemon: &Daemon) -> anyhow::Result<BaseViewModel> {
    let repos = daemon
        .config
        .repositories
        .0
        .iter()
        .map(|(name, _definition)| NavRepo {
            name: name.0.clone(),
            link: uri!(crate::repo::repo: name = &name.0).to_string(),
        })
        .collect::<Vec<_>>();

    let mut backups = Vec::new();
    for name in daemon.config.backups.0.keys() {
        let item = nav_backup_item(daemon, name).await;
        backups.push(item);
    }

    Ok(BaseViewModel {
        instance_name: daemon.instance_name.clone(),
        nav: NavViewModel {
            overview: NavItem {
                name: "Overview".to_string(),
                link: uri!(crate::index::index).to_string(),
            },
            repos,
            backups,
        },
    })
}
