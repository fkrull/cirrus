use futures::channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};
use futures::pin_mut;
use futures::SinkExt;
use std::future::Future;
use std::pin::Pin;

#[derive(Debug)]
pub struct MessageReceiver<M> {
    recv: UnboundedReceiver<M>,
}

impl<M> MessageReceiver<M> {
    pub async fn recv(&mut self) -> Result<Option<M>, ()> {
        todo!()
    }
}

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
    Error,
    IdleReady,
}

#[derive(Debug)]
pub struct ActorInstance<A: Actor> {
    actor_impl: A,
    recv: MessageReceiver<A::Message>,
}

impl<A: Actor> ActorInstance<A> {
    pub fn new(actor_impl: A) -> (ActorInstance<A>, ActorRef<A::Message>) {
        let (send, recv) = futures::channel::mpsc::unbounded();
        let actor_instance = ActorInstance {
            actor_impl,
            recv: MessageReceiver { recv },
        };
        let actor_ref = ActorRef { send };
        (actor_instance, actor_ref)
    }

    async fn select(&mut self) -> ActorSelect<A::Message> {
        use futures::future::select;
        use futures::future::Either;

        let recv_fut = self.recv.recv();
        let idle_fut = self.actor_impl.on_idle();
        pin_mut!(recv_fut);
        match select(recv_fut, idle_fut).await {
            Either::Left((Ok(Some(message)), _)) => ActorSelect::MessageReceived(message),
            Either::Left((Ok(None), _)) => ActorSelect::ChannelClosed,
            Either::Left((Err(_error), _)) => {
                // TODO error
                ActorSelect::Error
            }
            Either::Right((Ok(()), _)) => ActorSelect::IdleReady,
            Either::Right((Err(_error), _)) => {
                // TODO error
                ActorSelect::Error
            }
        }
    }

    pub async fn run(&mut self) -> Result<(), ()> {
        loop {
            match self.select().await {
                ActorSelect::MessageReceived(message) => {
                    // TODO error
                    self.actor_impl.on_message(message).await;
                }
                ActorSelect::ChannelClosed => {
                    // TODO error
                    self.actor_impl.on_close().await;
                    return Ok(());
                }
                ActorSelect::Error => {
                    // TODO error
                    todo!();
                }
                ActorSelect::IdleReady => {}
            };
        }
    }
}

#[derive(Debug, Clone)]
pub struct ActorRef<M> {
    send: UnboundedSender<M>,
}

impl<M> ActorRef<M> {
    pub async fn send(&mut self, message: M) -> Result<(), ()> {
        // TODO error
        let r = self.send.send(message).await;
        r.map_err(|_| ())
    }
}
