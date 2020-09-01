use crate::{assets::templates::Template, routes::nav::NavViewModel, routes::page::PageViewModel};
use cirrus_daemon::Daemon;
use rocket::{get, State};
use serde::Serialize;

pub(crate) mod page {
    use crate::routes::nav::NavViewModel;
    use serde::Serialize;

    #[derive(Debug, Serialize)]
    pub(crate) struct PageViewModel {
        pub pagename: String,
        pub hostname: String,
        pub nav: NavViewModel,
    }
}

pub(crate) mod nav {
    use serde::Serialize;

    #[derive(Debug, Serialize)]
    pub(crate) struct NavViewModel {
        pub repos: Vec<NavRepo>,
        pub backups: Vec<NavBackup>,
    }

    #[derive(Debug, Serialize)]
    pub(crate) struct NavRepo {
        pub name: String,
        pub link: String,
    }

    #[derive(Debug, Serialize)]
    pub(crate) enum BackupStatus {
        Idle,
        Running,
        Error,
    }

    #[derive(Debug, Serialize)]
    pub(crate) struct NavBackup {
        pub name: String,
        pub link: String,
        pub status: BackupStatus,
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct IndexViewModel {
    pub page: PageViewModel,
}

#[get("/")]
pub(crate) fn index(daemon: State<Daemon>) -> Template {
    Template::render(
        "index.html",
        IndexViewModel {
            page: PageViewModel {
                pagename: "Overview".to_string(),
                hostname: daemon.instance_name.clone(),
                nav: NavViewModel {
                    repos: vec![],
                    backups: vec![],
                },
            },
        },
    )
}
