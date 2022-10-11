use tokio::sync::oneshot;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Reason {
    Shutdown,
    Suspend,
}

#[derive(Debug)]
pub struct Request {
    pub reason: Reason,
    response_send: oneshot::Sender<()>,
}

impl Request {
    pub fn acknowledge(self) {
        if let Err(_) = self.response_send.send(()) {
            tracing::warn!("response receiver dropped");
        }
    }
}

#[derive(Debug)]
pub struct Send(oneshot::Sender<Request>);

impl Send {
    #[tracing::instrument(skip(self))]
    pub async fn cancel(self, reason: Reason) {
        let (response_send, response_recv) = oneshot::channel();
        let request = Request {
            reason,
            response_send,
        };
        if let Err(_) = self.0.send(request) {
            tracing::debug!("receiver dropped, job won't be cancelled");
        } else {
            if let Err(error) = response_recv.await {
                tracing::debug!(%error, "response sender dropped");
            }
        }
    }
}

#[derive(Debug)]
pub struct Recv(oneshot::Receiver<Request>);

impl Recv {
    pub async fn recv(&mut self) -> Request {
        match (&mut self.0).await {
            Ok(request) => request,
            Err(error) => {
                tracing::debug!(%error, "cancellation sender dropped, job will never be cancelled");
                futures::future::pending::<()>().await;
                unreachable!()
            }
        }
    }
}

pub fn new() -> (Send, Recv) {
    let (send, recv) = oneshot::channel();
    (Send(send), Recv(recv))
}
