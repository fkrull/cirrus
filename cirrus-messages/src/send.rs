use futures::channel::mpsc;
use std::convert::{TryFrom, TryInto};

#[derive(Debug, thiserror::Error)]
#[error("sending message failed")]
pub struct SendError(#[from] mpsc::SendError);

pub trait Sender<M> {
    fn send(&mut self, message: M) -> Result<(), SendError>;

    fn dyn_clone(&self) -> Box<dyn Sender<M>>;
}

pub struct SingleSender<M> {
    send: mpsc::UnboundedSender<M>,
}

impl<M> SingleSender<M> {
    pub(crate) fn new(send: mpsc::UnboundedSender<M>) -> Self {
        Self { send }
    }
}

impl<M> std::fmt::Debug for SingleSender<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SingleSender").finish()
    }
}

impl<M> Clone for SingleSender<M> {
    fn clone(&self) -> Self {
        Self {
            send: self.send.clone(),
        }
    }
}

impl<M: 'static> Sender<M> for SingleSender<M> {
    fn send(&mut self, message: M) -> Result<(), SendError> {
        self.send
            .unbounded_send(message)
            .map_err(|e| e.into_send_error())?;
        Ok(())
    }

    fn dyn_clone(&self) -> Box<dyn Sender<M>> {
        Box::new(self.clone())
    }
}

pub struct MultiSender<M> {
    sends: Vec<Box<dyn Sender<M>>>,
}

impl<M> Clone for MultiSender<M> {
    fn clone(&self) -> Self {
        Self {
            sends: self.sends.iter().map(|s| s.dyn_clone()).collect(),
        }
    }
}

impl<M> std::fmt::Debug for MultiSender<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MultiSender").finish()
    }
}

impl<M> MultiSender<M> {
    pub fn new() -> Self {
        MultiSender { sends: Vec::new() }
    }
}

impl<In: 'static> MultiSender<In> {
    pub fn connect<Out: TryFrom<In> + 'static, S: Sender<Out> + Clone + 'static>(
        &mut self,
        sender: S,
    ) {
        self.sends.push(Box::new(TryFromSender {
            sender,
            _ghost: Default::default(),
        }));
    }

    pub fn with<Out: TryFrom<In> + 'static, S: Sender<Out> + Clone + 'static>(
        mut self,
        sender: S,
    ) -> Self {
        self.connect(sender);
        self
    }
}

impl<M: Clone + 'static> Sender<M> for MultiSender<M> {
    fn send(&mut self, message: M) -> Result<(), SendError> {
        for send in &mut self.sends {
            send.send(message.clone())?;
        }
        Ok(())
    }

    fn dyn_clone(&self) -> Box<dyn Sender<M>> {
        Box::new(self.clone())
    }
}

struct TryFromSender<S, Out> {
    sender: S,
    _ghost: std::marker::PhantomData<Out>,
}

impl<S, Out> std::fmt::Debug for TryFromSender<S, Out> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TryFromSender").finish()
    }
}

impl<In: 'static, Out: TryFrom<In> + 'static, S: Sender<Out> + Clone + 'static> Sender<In>
    for TryFromSender<S, Out>
{
    fn send(&mut self, message: In) -> Result<(), SendError> {
        match message.try_into() {
            Ok(message) => self.sender.send(message),
            Err(_) => Ok(()),
        }
    }

    fn dyn_clone(&self) -> Box<dyn Sender<In>> {
        Box::new(TryFromSender {
            sender: self.sender.clone(),
            _ghost: Default::default(),
        })
    }
}
