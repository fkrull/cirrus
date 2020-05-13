#![feature(proc_macro_hygiene, decl_macro)]
use anyhow::anyhow;
use rocket::{get, routes};
use std::sync::{Arc, RwLock};

pub mod config;
pub mod scheduler;

#[derive(Debug)]
pub struct PauseState {
    paused: RwLock<bool>,
}

impl PauseState {
    pub fn new() -> Self {
        Self {
            paused: RwLock::new(true),
        }
    }

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
    pub repositories: config::Repositories,
    pub backups: config::Backups,
}

#[get("/")]
fn index() -> &'static str {
    "hello world"
}

fn main() -> anyhow::Result<()> {
    let config_file = dirs::config_dir()
        .ok_or_else(|| anyhow!("can't find config directory"))?
        .join("restic-controller")
        .join("config.toml");
    let cfg_data = std::fs::read_to_string(config_file)?;
    let cfg: config::Config = toml::from_str(&cfg_data)?;
    let app = Arc::new(App {
        pause_state: PauseState::new(),
        repositories: cfg.repositories,
        backups: cfg.backups,
    });

    // TODO: handle panics in scheduler thread
    scheduler::start_scheduler(app)?;
    rocket::ignite().mount("/", routes![index]).launch();

    Ok(())
}
