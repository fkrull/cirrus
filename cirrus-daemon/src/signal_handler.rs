use crate::shutdown::RequestShutdown;
use shindig::{Events, Sender};

#[derive(Debug)]
pub struct SignalHandler {
    sender: Sender,
}

impl SignalHandler {
    pub fn new(events: &mut Events) -> Self {
        SignalHandler {
            sender: events.sender(),
        }
    }

    #[tracing::instrument(name = "SignalHandler", skip_all)]
    pub async fn run(&mut self) -> eyre::Result<()> {
        let signal = shutdown_signals().await?;
        tracing::info!(?signal, "exiting due to signal");
        self.sender.send(RequestShutdown);
        Ok(())
    }
}

#[derive(Debug)]
enum Signal {
    #[cfg(unix)]
    SIGALRM,
    #[cfg(unix)]
    SIGHUP,
    #[cfg(unix)]
    SIGINT,
    #[cfg(unix)]
    SIGPIPE,
    #[cfg(unix)]
    SIGTERM,
    #[cfg(unix)]
    SIGUSR1,
    #[cfg(unix)]
    SIGUSR2,

    #[cfg(windows)]
    CtrlBreak,
    #[cfg(windows)]
    CtrlC,
    #[cfg(windows)]
    CtrlClose,
    #[cfg(windows)]
    CtrlLogoff,
    #[cfg(windows)]
    CtrlShutdown,
}

#[cfg(unix)]
async fn shutdown_signals() -> eyre::Result<Signal> {
    use tokio::signal::unix::SignalKind;

    let mut alarm = tokio::signal::unix::signal(SignalKind::alarm())?;
    let mut hangup = tokio::signal::unix::signal(SignalKind::hangup())?;
    let mut interrupt = tokio::signal::unix::signal(SignalKind::interrupt())?;
    let mut pipe = tokio::signal::unix::signal(SignalKind::pipe())?;
    let mut terminate = tokio::signal::unix::signal(SignalKind::terminate())?;
    let mut user_defined1 = tokio::signal::unix::signal(SignalKind::user_defined1())?;
    let mut user_defined2 = tokio::signal::unix::signal(SignalKind::user_defined2())?;
    let signal = tokio::select! {
        _ = alarm.recv() => Signal::SIGALRM,
        _ = hangup.recv() => Signal::SIGHUP,
        _ = interrupt.recv() => Signal::SIGINT,
        _ = pipe.recv() => Signal::SIGPIPE,
        _ = terminate.recv() => Signal::SIGTERM,
        _ = user_defined1.recv() => Signal::SIGUSR1,
        _ = user_defined2.recv() => Signal::SIGUSR2,
    };
    Ok(signal)
}

#[cfg(windows)]
async fn shutdown_signals() -> eyre::Result<Signal> {
    let mut ctrl_break = tokio::signal::windows::ctrl_break()?;
    let mut ctrl_c = tokio::signal::windows::ctrl_c()?;
    let mut ctrl_close = tokio::signal::windows::ctrl_close()?;
    let mut ctrl_logoff = tokio::signal::windows::ctrl_logoff()?;
    let mut ctrl_shutdown = tokio::signal::windows::ctrl_shutdown()?;
    let signal = tokio::select! {
        _ = ctrl_break.recv() => Signal::CtrlBreak,
        _ = ctrl_c.recv() => Signal::CtrlC,
        _ = ctrl_close.recv() => Signal::CtrlClose,
        _ = ctrl_logoff.recv() => Signal::CtrlLogoff,
        _ = ctrl_shutdown.recv() => Signal::CtrlShutdown,
    };
    Ok(signal)
}
