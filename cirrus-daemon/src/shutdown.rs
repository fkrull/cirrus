use events::{Events, Sender, Subscriber};
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct RequestShutdown;

#[derive(Debug, Clone)]
pub struct ShutdownRequested {
    pub grace_deadline: Instant,
}

#[derive(Debug, Clone)]
pub struct ShutdownAcknowledged;

const SHUTDOWN_GRACE_PERIOD: Duration = Duration::from_secs(5);

fn shutdown() -> ! {
    tracing::info!("shutting down");
    std::process::exit(0);
}

#[derive(Debug)]
pub struct ShutdownService {
    sender: Sender,
    sub_request_shutdown: Subscriber<RequestShutdown>,
    sub_shutdown_acknowledged: Subscriber<ShutdownAcknowledged>,
}

impl ShutdownService {
    pub fn new(events: &mut Events) -> Self {
        ShutdownService {
            sender: events.sender(),
            sub_request_shutdown: events.subscribe(),
            sub_shutdown_acknowledged: events.subscribe(),
        }
    }

    #[tracing::instrument(name = "ShutdownService", skip_all)]
    pub async fn run(&mut self) -> eyre::Result<()> {
        let _ = self.sub_request_shutdown.recv().await?;
        tracing::info!(
            grace_period_secs = SHUTDOWN_GRACE_PERIOD.as_secs_f64(),
            "shutdown requested"
        );
        let grace_deadline = Instant::now() + SHUTDOWN_GRACE_PERIOD;
        let mut required_acks = self.sender.send(ShutdownRequested { grace_deadline });
        loop {
            tokio::select! {
                ack = self.sub_shutdown_acknowledged.recv() => {
                    let _ = ack?;
                    required_acks -= 1;
                    tracing::debug!(required_acks, "received ack");
                    if required_acks == 0 {
                        shutdown();
                    }
                }
                _ = tokio::time::sleep_until(tokio::time::Instant::from_std(grace_deadline)) => {
                    tracing::warn!("grace period elapsed before all receivers acknowledged shutdown");
                    shutdown();
                }
            }
        }
    }
}
