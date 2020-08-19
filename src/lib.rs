//use crate::{jobs::JobsRepo, pause::PauseState};

use crate::model::Config;
use crate::restic::Restic;
use crate::secrets::Secrets;
use std::sync::Arc;

pub mod commands;
pub mod jobs;
pub mod model;
//pub mod pause;
pub mod restic;
//pub mod scheduler;
pub mod secrets;

pub type Timestamp = chrono::DateTime<chrono::Utc>;

#[derive(Debug)]
pub struct Cirrus {
    pub config: Arc<Config>,
    pub restic: Arc<Restic>,
    pub secrets: Arc<Secrets>,
}
