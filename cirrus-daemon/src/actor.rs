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

    fn on_idle(&mut self) -> Pin<Box<dyn Future<Output = ()> + '_>> {
        Box::pin(futures::future::pending())
    }

    /*fn on_recv(
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
    }*/
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

    /*fn on_recv(
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
    }*/

    pub async fn run(&mut self) {
        use futures::future::select;
        use futures::future::Either;

        loop {
            let recv_fut = self.recv.recv();
            let idle_fut = self.actor_impl.on_idle();
            pin_mut!(recv_fut);
            let s = select(recv_fut, idle_fut);

            let msg = match s.await {
                Either::Left((Ok(Some(message)), _)) => {
                    Some(message)
                    //self.actor_impl.on_message(message).await;
                }
                Either::Left((Ok(None), _)) => None,
                Either::Left((Err(e), _)) => todo!(),
                Either::Right(_) => todo!(),
            };
            match msg {
                Some(msg) => {
                    self.actor_impl.on_message(msg).await;
                }
                None => {
                    self.actor_impl.on_close().await;
                    break;
                }
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
        let r = self.send.send(message).await;
        r.map_err(|_| ())
    }
}
