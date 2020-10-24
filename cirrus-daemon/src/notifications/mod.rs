use crate::job::JobStatusChange;

#[derive(Debug)]
pub struct Notifications;

impl Notifications {
    pub fn new() -> Self {
        Notifications
    }
}

#[async_trait::async_trait]
impl cirrus_actor::Actor for Notifications {
    type Message = JobStatusChange;
    type Error = eyre::Report;

    async fn on_message(&mut self, _message: Self::Message) -> Result<(), Self::Error> {
        Ok(())
    }
}
