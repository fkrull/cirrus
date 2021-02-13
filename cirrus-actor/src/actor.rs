use crate::SendError;
use futures::channel::mpsc;

#[async_trait::async_trait]
pub trait Actor: Send {
    type Message;
    type Error;

    async fn on_message(&mut self, message: Self::Message) -> Result<(), Self::Error>;

    async fn on_start(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn on_close(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn on_idle(&mut self) -> Result<(), Self::Error> {
        futures::future::pending::<()>().await;
        unreachable!()
    }
}

#[derive(Debug)]
enum ActorSelect<M> {
    MessageReceived(M),
    ChannelClosed,
    IdleReady,
}

#[derive(Debug)]
pub struct ActorInstance<A: Actor> {
    actor_impl: A,
    recv: mpsc::UnboundedReceiver<A::Message>,
}

impl<A: Actor> ActorInstance<A> {
    pub async fn run(&mut self) -> Result<(), A::Error> {
        self.actor_impl.on_start().await?;
        loop {
            match self.select().await? {
                ActorSelect::MessageReceived(message) => {
                    self.actor_impl.on_message(message).await?;
                }
                ActorSelect::ChannelClosed => {
                    self.actor_impl.on_close().await?;
                    break;
                }
                ActorSelect::IdleReady => {}
            };
        }

        Ok(())
    }

    async fn select(&mut self) -> Result<ActorSelect<A::Message>, A::Error> {
        use futures::{future::select, future::Either, pin_mut, stream::StreamExt};

        let recv_fut = self.recv.next();
        let idle_fut = self.actor_impl.on_idle();
        pin_mut!(recv_fut);
        match select(recv_fut, idle_fut).await {
            Either::Left((Some(message), _)) => Ok(ActorSelect::MessageReceived(message)),
            Either::Left((None, _)) => Ok(ActorSelect::ChannelClosed),
            Either::Right((Ok(()), _)) => Ok(ActorSelect::IdleReady),
            Either::Right((Err(error), _)) => Err(error),
        }
    }
}

#[derive(Debug)]
pub struct ActorRef<M> {
    send: mpsc::UnboundedSender<M>,
}

impl<M> Clone for ActorRef<M> {
    fn clone(&self) -> Self {
        ActorRef {
            send: self.send.clone(),
        }
    }
}

impl<M> ActorRef<M> {
    pub fn send(&mut self, message: M) -> Result<(), SendError> {
        self.send
            .unbounded_send(message)
            .map_err(|e| e.into_send_error())?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct ActorBuilder<M> {
    actor_ref: ActorRef<M>,
    recv: mpsc::UnboundedReceiver<M>,
}

impl<M> ActorBuilder<M> {
    pub fn actor_ref(&self) -> ActorRef<M> {
        self.actor_ref.clone()
    }

    pub fn into_instance<A: Actor<Message = M>>(self, actor_impl: A) -> ActorInstance<A> {
        ActorInstance {
            recv: self.recv,
            actor_impl,
        }
    }
}

pub fn new<M>() -> ActorBuilder<M> {
    let (send, recv) = futures::channel::mpsc::unbounded();
    let actor_ref = ActorRef { send };
    ActorBuilder { actor_ref, recv }
}
