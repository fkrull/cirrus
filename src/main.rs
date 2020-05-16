#![feature(proc_macro_hygiene, decl_macro, try_find)]

use crate::jobs::JobsRepo;
use anyhow::anyhow;
use env_logger::Env;
use rocket::{get, routes};
use std::{
    path::PathBuf,
    sync::{Arc, RwLock},
};

pub mod config;
pub mod jobs;
pub mod scheduler;

#[derive(Debug, Default)]
pub struct PauseState {
    paused: RwLock<bool>,
}

impl PauseState {
    pub fn pause(&self) {
        self.set_paused(false);
    }

    pub fn resume(&self) {
        self.set_paused(true);
    }

    pub fn paused(&self) -> bool {
        *self.paused.read().unwrap()
    }

    fn set_paused(&self, paused: bool) {
        *self.paused.write().unwrap() = paused;
    }
}

#[derive(Debug)]
pub struct App {
    pub pause_state: PauseState,
    pub jobs: JobsRepo,
    pub repositories: config::Repositories,
}

#[get("/")]
fn index() -> &'static str {
    "hello world"
}

fn default_config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|dir| dir.join("restic-controller").join("config.toml"))
}

fn config_path() -> anyhow::Result<PathBuf> {
    std::env::var_os("RESTIC_CONTROLLER_CONFIG")
        .map(PathBuf::from)
        .or_else(default_config_path)
        .ok_or_else(|| anyhow!("can't find config file"))
}

fn main() -> anyhow::Result<()> {
    env_logger::from_env(Env::default().default_filter_or("info")).init();
    let cfg_data = std::fs::read_to_string(config_path()?)?;
    let cfg: config::Config = toml::from_str(&cfg_data)?;
    let app = Arc::new(App {
        pause_state: PauseState::default(),
        jobs: JobsRepo::new(cfg.backups.0),
        repositories: cfg.repositories,
    });

    // TODO: handle panics in scheduler thread
    scheduler::start_scheduler(app)?;
    rocket::ignite().mount("/", routes![index]).launch();

    Ok(())
}
