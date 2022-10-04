use std::any::{Any, TypeId};
use std::collections::HashMap;
use tokio::sync::broadcast;

#[derive(Debug)]
struct Channel<T>(broadcast::Sender<T>);

#[derive(Debug)]
struct ErasedChannel(Box<dyn Any>);

impl ErasedChannel {
    fn new<T: Clone + 'static>() -> Self {
        let (send, _) = broadcast::channel::<T>(100);
        ErasedChannel(Box::new(send))
    }

    fn send<T: 'static>(&self, value: T) {
        // TODO error handling, logging?
        self.0
            .downcast_ref::<broadcast::Sender<T>>()
            .unwrap()
            .send(value)
            .map_err(|_| "oh no")
            .unwrap();
    }
}

#[derive(Debug)]
pub struct EventsBuilder {
    channels: HashMap<TypeId, ErasedChannel>,
}

impl EventsBuilder {
    pub fn new() -> Self {
        EventsBuilder {
            channels: HashMap::new(),
        }
    }

    pub fn with_type<T: Clone + 'static>(&mut self) -> &mut Self {
        let channel = ErasedChannel::new::<T>();
        self.channels.insert(TypeId::of::<T>(), channel);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
}
