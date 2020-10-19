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

    fn on_message(&mut self, message: Self::Message) -> Pin<Box<dyn Future<Output = ()> + '_>>;

    fn on_close(&mut self) -> Pin<Box<dyn Future<Output = ()> + '_>> {
        Box::pin(futures::future::ready(()))
    }

    fn on_recv(
        &mut self,
        message: Option<Self::Message>,
    ) -> Pin<Box<dyn Future<Output = bool> + '_>> {
        Box::pin(async move {
            match message {
                Some(message) => {
                    self.on_message(message).await;
                    true
                }
                None => {
                    self.on_close().await;
                    false
                }
            }
        })
    }

    fn run<'a>(
        &'a mut self,
        recv: &'a mut MessageReceiver<Self::Message>,
    ) -> Pin<Box<dyn Future<Output = ()> + 'a>> {
        Box::pin(async move {
            loop {
                let message = recv.recv().await.unwrap();
                if !self.on_recv(message).await {
                    break;
                }
            }
        })
    }
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

    pub async fn run(&mut self) {
        self.actor_impl.run(&mut self.recv).await;
    }
}

#[derive(Debug, Clone)]
pub struct ActorRef<M> {
    send: UnboundedSender<M>,
}

impl<M> ActorRef<M> {
    pub async fn send(&mut self, message: M) -> Result<(), ()> {
        let r = self.send.send(message).await;
        r.map_err(|_| ())
    }
}
