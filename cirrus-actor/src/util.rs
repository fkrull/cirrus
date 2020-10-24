use crate::{Actor, ActorRef};

#[derive(Debug)]
pub struct NullSink<M> {
    ghost: std::marker::PhantomData<M>,
}

impl<M> Default for NullSink<M> {
    fn default() -> Self {
        NullSink {
            ghost: Default::default(),
        }
    }
}

impl<M> NullSink<M> {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait::async_trait]
impl<M: Send> Actor for NullSink<M> {
    type Message = M;
    type Error = std::convert::Infallible;

    async fn on_message(&mut self, _message: Self::Message) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[derive(Debug)]
pub struct MultiplexActor<M> {
    sinks: Vec<ActorRef<M>>,
}

impl<M> Default for MultiplexActor<M> {
    fn default() -> Self {
        MultiplexActor { sinks: Vec::new() }
    }
}

impl<M> MultiplexActor<M> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn new_with(sinks: impl Into<Vec<ActorRef<M>>>) -> Self {
        let sinks = sinks.into();
        MultiplexActor { sinks }
    }

    pub fn add(&mut self, sink: ActorRef<M>) {
        self.sinks.push(sink);
    }
}

#[async_trait::async_trait]
impl<M: Send + Clone> Actor for MultiplexActor<M> {
    type Message = M;
    type Error = crate::SendError;

    async fn on_message(&mut self, message: Self::Message) -> Result<(), Self::Error> {
        for sink in &mut self.sinks {
            sink.send(message.clone())?;
        }
        Ok(())
    }
}
