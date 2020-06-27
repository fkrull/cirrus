use crate::{jobs::JobsRepo, pause::PauseState};

pub mod jobs;
pub mod model;
pub mod pause;
pub mod restic;
pub mod scheduler;

#[derive(Debug, Default)]
pub struct App {
    pub pause_state: PauseState,
    pub jobs: JobsRepo,
    pub repositories: model::Repositories,
}
