#![feature(proc_macro_hygiene, decl_macro)]

use anyhow::anyhow;
//use cirrus::{jobs::JobsRepo, model, pause::PauseState, scheduler, App};
use env_logger::Env;
//use rocket::{get, routes};
use cirrus::model;
use std::path::PathBuf;

fn default_config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|dir| dir.join("cirrus").join("config.toml"))
}

fn config_path() -> anyhow::Result<PathBuf> {
    std::env::var_os("CIRRUS_CONFIG")
        .map(PathBuf::from)
        .or_else(default_config_path)
        .ok_or_else(|| anyhow!("can't find config file"))
}

fn main() -> anyhow::Result<()> {
    env_logger::from_env(Env::default().default_filter_or("info")).init();
    let cfg_data = std::fs::read_to_string(config_path()?)?;
    let _config: model::Config = toml::from_str(&cfg_data)?;
    /*let app = Arc::new(App {
        pause_state: PauseState::default(),
        jobs: JobsRepo::new(cfg.backups.0),
        repositories: cfg.repositories,
    });

    // TODO: handle panics in scheduler thread
    scheduler::start_scheduler(app)?;

    #[cfg(feature = "desktop-integration")]
    if let Err(err) = webbrowser::open("http://localhost:8000") {
        log::error!("failed to open web browser: {:?}", err);
    }

    rocket::ignite().mount("/", routes![index]).launch();*/

    Ok(())
}
