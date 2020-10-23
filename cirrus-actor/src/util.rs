use crate::Actor;

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
