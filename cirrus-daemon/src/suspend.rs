use shindig::Events;

#[derive(Debug, Copy, Clone)]
pub enum Suspend {
    UntilDisabled,
    // TODO: suspend until a certain time
}

#[derive(Debug, Copy, Clone)]
pub struct Unsuspend;

#[derive(Debug)]
pub struct SuspendService {
    events: Events,
    suspended: Option<Suspend>,
}

impl SuspendService {
    pub fn new(events: Events) -> Self {
        // TODO: save and restore suspended status
        SuspendService {
            events,
            suspended: None,
        }
    }

    #[tracing::instrument(name = "SuspendService", skip_all)]
    pub async fn run(&mut self) -> eyre::Result<()> {
        // TODO: should I subscribe in new(), so there's no race of tasks sending out events before everything is subscribed?
        let mut suspend_recv = self.events.subscribe::<Suspend>();
        let mut unsuspend_recv = self.events.subscribe::<Unsuspend>();
        loop {
            tokio::select! {
                suspend = suspend_recv.recv() => {
                    let suspend = suspend?;
                    tracing::info!(?suspend, "suspended");
                    self.suspended = Some(suspend);
                },
                unsuspend = unsuspend_recv.recv() => {
                    let _ = unsuspend?;
                    tracing::info!("unsuspended");
                    self.suspended = None;
                }
            }
        }
    }
}
