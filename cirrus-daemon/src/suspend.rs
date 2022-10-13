#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Suspend {
    UntilDisabled,
    NotSuspended,
}

impl Default for Suspend {
    fn default() -> Self {
        Suspend::NotSuspended
    }
}

impl Suspend {
    pub fn is_suspended(&self) -> bool {
        match self {
            Suspend::UntilDisabled => true,
            Suspend::NotSuspended => false,
        }
    }

    pub fn toggle(&self) -> Suspend {
        match self {
            Suspend::UntilDisabled => Suspend::NotSuspended,
            Suspend::NotSuspended => Suspend::UntilDisabled,
        }
    }
}

events::subscriptions! {
    Suspend,
}

#[derive(Debug)]
pub struct SuspendService {
    suspend: Suspend,
    events: Subscriptions,
}

impl SuspendService {
    pub fn new(events: &mut events::Builder) -> Self {
        // TODO: save and restore suspended status
        SuspendService {
            suspend: Suspend::default(),
            events: Subscriptions::subscribe(events),
        }
    }

    #[tracing::instrument(name = "SuspendService", skip_all)]
    pub async fn run(&mut self) -> eyre::Result<()> {
        loop {
            let suspend = self.events.Suspend.recv().await?;
            // TODO: save state
            if !self.suspend.is_suspended() && suspend.is_suspended() {
                tracing::info!(?suspend, "suspended");
            } else if self.suspend.is_suspended() && !suspend.is_suspended() {
                tracing::info!(?suspend, "unsuspended");
            }
            self.suspend = suspend;
        }
    }

    pub fn get_suspend(&self) -> &Suspend {
        &self.suspend
    }
}
