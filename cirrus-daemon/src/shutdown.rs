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

fn shutdown(graceful: bool) {
    if !graceful {
        tracing::warn!("grace period elapsed before all receivers acknowledged shutdown");
    }
    tracing::info!("shutting down");
    std::process::exit(0);
}

events::subscriptions! {
    RequestShutdown,
    ShutdownAcknowledged,
}

pub struct ShutdownService {
    events: Subscriptions,
    grace_period: Duration,
    shutdown: Box<dyn FnMut(bool) + Send>,
}

impl ShutdownService {
    pub fn new(events: &mut events::Builder) -> Self {
        Self::new_internal(events, SHUTDOWN_GRACE_PERIOD, Box::new(shutdown))
    }

    fn new_internal(
        events: &mut events::Builder,
        grace_period: Duration,
        shutdown: Box<dyn FnMut(bool) + Send>,
    ) -> Self {
        ShutdownService {
            events: Subscriptions::subscribe(events),
            grace_period,
            shutdown,
        }
    }

    #[tracing::instrument(name = "ShutdownService", skip_all)]
    pub async fn run(&mut self) -> eyre::Result<()> {
        let _ = self.events.RequestShutdown.recv().await?;
        tracing::info!(
            grace_period_secs = self.grace_period.as_secs_f64(),
            "shutdown requested"
        );
        let grace_deadline = Instant::now() + self.grace_period;
        let mut required_acks = self.events.send(ShutdownRequested { grace_deadline });
        if required_acks == 0 {
            (self.shutdown)(true);
        }
        loop {
            tokio::select! {
                ack = self.events.ShutdownAcknowledged.recv() => {
                    let _ = ack?;
                    required_acks -= 1;
                    tracing::debug!(required_acks, "received ack");
                    if required_acks == 0 {
                        (self.shutdown)(true);
                    }
                }
                _ = tokio::time::sleep_until(tokio::time::Instant::from_std(grace_deadline)) => {
                    (self.shutdown)(false);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn should_exit_gracefully() {
        let mut events = events::Builder::new_with_capacity(10);
        let (send, mut recv) = tokio::sync::mpsc::unbounded_channel();
        let mut shutdown_service = ShutdownService::new_internal(
            &mut events,
            Duration::from_nanos(1),
            Box::new(move |graceful| {
                send.send(graceful).unwrap();
                panic!("done");
            }),
        );

        let join = tokio::spawn(async move { shutdown_service.run().await });

        events.typed_sender().send(RequestShutdown);
        let graceful = recv.recv().await.unwrap();
        assert!(graceful);
        join.await.unwrap_err();
    }

    #[tokio::test]
    async fn should_exit_gracefully_once_all_acked() {
        let mut events = events::Builder::new_with_capacity(10);
        let (send, mut recv) = tokio::sync::mpsc::unbounded_channel();
        let mut shutdown_service = ShutdownService::new_internal(
            &mut events,
            Duration::from_secs(5),
            Box::new(move |graceful| {
                send.send(graceful).unwrap();
                panic!("done");
            }),
        );
        let mut sub1 = events.subscribe::<ShutdownRequested>();
        let mut sub2 = events.subscribe::<ShutdownRequested>();

        let join = tokio::spawn(async move { shutdown_service.run().await });

        events.typed_sender().send(RequestShutdown);
        sub1.recv().await.unwrap();
        events.typed_sender().send(ShutdownAcknowledged);
        sub2.recv().await.unwrap();
        events.typed_sender().send(ShutdownAcknowledged);
        let graceful = recv.recv().await.unwrap();
        assert!(graceful);
        join.await.unwrap_err();
    }

    #[tokio::test]
    async fn should_exit_ungracefully_if_missing_ack() {
        let mut events = events::Builder::new_with_capacity(10);
        let (send, mut recv) = tokio::sync::mpsc::unbounded_channel();
        let mut shutdown_service = ShutdownService::new_internal(
            &mut events,
            Duration::from_millis(50),
            Box::new(move |graceful| {
                send.send(graceful).unwrap();
                panic!("done");
            }),
        );
        let mut sub1 = events.subscribe::<ShutdownRequested>();
        let mut sub2 = events.subscribe::<ShutdownRequested>();

        let join = tokio::spawn(async move { shutdown_service.run().await });

        events.typed_sender().send(RequestShutdown);
        sub1.recv().await.unwrap();
        events.typed_sender().send(ShutdownAcknowledged);
        sub2.recv().await.unwrap();
        // no second ack
        let graceful = recv.recv().await.unwrap();
        assert!(!graceful);
        join.await.unwrap_err();
    }
}
