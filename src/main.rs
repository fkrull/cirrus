#![feature(proc_macro_hygiene, decl_macro)]
use rocket::{get, routes};
use std::sync::RwLock;

pub mod config;

#[derive(Debug)]
pub struct PauseState {
    running: RwLock<bool>,
}

impl PauseState {
    pub fn pause(&self) {
        self.set_running(false);
    }

    pub fn resume(&self) {
        self.set_running(true);
    }

    pub fn running(&self) -> bool {
        *self.running.read().unwrap()
    }

    fn set_running(&self, running: bool) {
        *self.running.write().unwrap() = running;
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

fn main() {
    rocket::ignite().mount("/", routes![index]).launch();
}
