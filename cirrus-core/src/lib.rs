pub mod model;
pub mod restic;
pub mod secrets;

pub type Timestamp = chrono::DateTime<chrono::Utc>;

pub mod timestamp {
    pub fn now() -> crate::Timestamp {
        chrono::Utc::now()
    }
}
