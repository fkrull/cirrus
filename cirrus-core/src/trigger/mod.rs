use serde::{Deserialize, Serialize};

pub mod cron;

#[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Trigger {
    Cron(cron::Cron),
}

impl Trigger {
    pub fn next_schedule(&self, after: time::OffsetDateTime) -> eyre::Result<time::OffsetDateTime> {
        match self {
            Trigger::Cron(cron) => cron.next_schedule(after),
        }
    }
}
