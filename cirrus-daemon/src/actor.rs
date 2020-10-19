use futures::channel::mpsc;
use std::{future::Future, pin::Pin};

// TODO async-trait
pub trait Actor {
    type Message;
    type Error;

    fn on_message(
        &mut self,
        message: Self::Message,
    ) -> Pin<Box<dyn Future<Output = Result<(), Self::Error>> + '_>>;

    fn on_close(&mut self) -> Pin<Box<dyn Future<Output = Result<(), Self::Error>> + '_>> {
        Box::pin(futures::future::ready(Ok(())))
    }

    fn on_idle(&mut self) -> Pin<Box<dyn Future<Output = Result<(), Self::Error>> + '_>> {
        Box::pin(futures::future::pending())
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
    pub fn new(actor_impl: A) -> (ActorInstance<A>, ActorRef<A::Message>) {
        let (send, recv) = futures::channel::mpsc::unbounded();
        let actor_instance = ActorInstance { actor_impl, recv };
        let actor_ref = ActorRef { send };
        (actor_instance, actor_ref)
    }

    async fn select(&mut self) -> Result<ActorSelect<A::Message>, A::Error> {
        use futures::{
            future::{select, Either},
            pin_mut,
            stream::StreamExt,
        };

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

    pub async fn run(&mut self) -> Result<(), A::Error> {
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
}

#[derive(Debug, Clone)]
pub struct ActorRef<M> {
    send: mpsc::UnboundedSender<M>,
}

// TODO derive Error
#[derive(Debug)]
pub struct SendError(mpsc::SendError);

impl<M> ActorRef<M> {
    pub async fn send(&mut self, message: M) -> Result<(), SendError> {
        use futures::sink::SinkExt;
        self.send.send(message).await.map_err(|err| SendError(err))
    }
}
