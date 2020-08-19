//use crate::{jobs::JobsRepo, pause::PauseState};

pub mod commands;
pub mod jobs;
pub mod model;
//pub mod pause;
pub mod restic;
//pub mod scheduler;
pub mod secrets;

pub type Timestamp = chrono::DateTime<chrono::Utc>;

pub(crate) mod timestamp {
    pub fn now() -> crate::Timestamp {
        chrono::Utc::now()
    }
}
