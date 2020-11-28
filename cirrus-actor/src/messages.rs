use crate::{ActorRef, SendError};
use std::convert::{TryFrom, TryInto};

pub struct Messages<M>(Inner<M>);

enum Inner<M> {
    Discarding,
    ActorRef(ActorRef<M>),
    Dyn(Box<dyn DynMessages<M>>),
    Multicast(Vec<Messages<M>>),
}

impl<M> Messages<M> {
    pub fn new_discarding() -> Self {
        Self(Inner::Discarding)
    }
}

impl<M: Clone + Send + 'static> Messages<M> {
    pub fn send(&mut self, message: M) -> Result<(), SendError> {
        match &mut self.0 {
            Inner::Discarding => Ok(()),
            Inner::ActorRef(actor_ref) => actor_ref.send(message),
            Inner::Dyn(inner) => inner.send(message),
            Inner::Multicast(messages) => {
                for msg in messages {
                    msg.send(message.clone())?;
                }
                Ok(())
            }
        }
    }

    pub fn upcast<Super>(self) -> Messages<Super>
    where
        M: From<Super>,
    {
        match &self.0 {
            Inner::Discarding => Messages(Inner::Discarding),
            _ => Messages(Inner::Dyn(Box::new(FromMessages(self)))),
        }
    }

    pub fn upcast_filter<Super>(self) -> Messages<Super>
    where
        M: TryFrom<Super>,
    {
        match &self.0 {
            Inner::Discarding => Messages(Inner::Discarding),
            _ => Messages(Inner::Dyn(Box::new(TryFromMessages(self)))),
        }
    }

    pub fn also_to(mut self, other: Messages<M>) -> Messages<M> {
        match &mut self.0 {
            Inner::Discarding => other,
            Inner::Multicast(inner) => {
                inner.push(other);
                self
            }
            _ => Messages(Inner::Multicast(vec![self, other])),
        }
    }
}

impl<M> Default for Messages<M> {
    fn default() -> Self {
        Self::new_discarding()
    }
}

impl<M> Clone for Messages<M> {
    fn clone(&self) -> Self {
        match &self.0 {
            Inner::Discarding => Self(Inner::Discarding),
            Inner::ActorRef(inner) => Self(Inner::ActorRef(inner.clone())),
            Inner::Dyn(inner) => Self(Inner::Dyn(inner.dyn_clone())),
            Inner::Multicast(inner) => Self(Inner::Multicast(inner.clone())),
        }
    }
}

impl<M> std::fmt::Debug for Messages<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match &self.0 {
            Inner::Discarding => "Messages::Discarding",
            Inner::ActorRef(_) => "Messages::ActorRef",
            Inner::Dyn(_) => "Messages::Dyn",
            Inner::Multicast(_) => "Messages::Multicast",
        };
        f.debug_tuple(name).finish()
    }
}

impl<M> From<ActorRef<M>> for Messages<M> {
    fn from(actor_ref: ActorRef<M>) -> Self {
        Messages(Inner::ActorRef(actor_ref))
    }
}

trait DynMessages<M>: Send {
    fn send(&mut self, message: M) -> Result<(), SendError>;

    fn dyn_clone(&self) -> Box<dyn DynMessages<M>>;
}

struct FromMessages<Out>(Messages<Out>);

impl<In, Out: From<In> + Clone + Send + 'static> DynMessages<In> for FromMessages<Out> {
    fn send(&mut self, message: In) -> Result<(), SendError> {
        self.0.send(message.into())
    }

    fn dyn_clone(&self) -> Box<dyn DynMessages<In>> {
        Box::new(Self(self.0.clone()))
    }
}

struct TryFromMessages<Out>(Messages<Out>);

impl<In, Out: TryFrom<In> + Clone + Send + 'static> DynMessages<In> for TryFromMessages<Out> {
    fn send(&mut self, message: In) -> Result<(), SendError> {
        match message.try_into() {
            Ok(message) => self.0.send(message),
            Err(_) => Ok(()),
        }
    }

    fn dyn_clone(&self) -> Box<dyn DynMessages<In>> {
        Box::new(Self(self.0.clone()))
    }
}
