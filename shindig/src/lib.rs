use parking_lot::RwLock;
use std::{
    any::{type_name, Any, TypeId},
    collections::HashMap,
    sync::Arc,
};
use tokio::sync::broadcast;

#[derive(Debug, thiserror::Error)]
#[error("ErasedChannel was of type {actual} but was expected to be of type {expected}")]
struct ErasedTypeError {
    expected: &'static str,
    actual: &'static str,
}

#[derive(Debug)]
struct ErasedChannel {
    sender: Box<dyn Any>,
    type_name: &'static str,
}

impl ErasedChannel {
    fn new<T: Clone + 'static>(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel::<T>(capacity);
        ErasedChannel {
            sender: Box::new(sender),
            type_name: type_name::<T>(),
        }
    }

    fn clone<T: 'static>(&self) -> Result<Self, ErasedTypeError> {
        let sender = self.sender::<T>()?;
        Ok(ErasedChannel {
            sender: Box::new(sender.clone()),
            type_name: self.type_name,
        })
    }

    fn send<T: 'static>(&self, item: T) -> Result<usize, ErasedTypeError> {
        let sender = self.sender()?;
        Ok(sender.send(item).unwrap_or(0))
    }

    fn subscribe<T: 'static>(&self) -> Result<broadcast::Receiver<T>, ErasedTypeError> {
        let sender = self.sender()?;
        Ok(sender.subscribe())
    }

    fn sender<T: 'static>(&self) -> Result<&broadcast::Sender<T>, ErasedTypeError> {
        self.sender
            .downcast_ref::<broadcast::Sender<T>>()
            .ok_or_else(|| ErasedTypeError {
                expected: type_name::<T>(),
                actual: self.type_name,
            })
    }
}

#[derive(Debug)]
struct EventsShared {
    capacity: usize,
    channels: RwLock<HashMap<TypeId, ErasedChannel>>,
}

impl EventsShared {
    fn new(capacity: usize) -> Self {
        EventsShared {
            capacity,
            channels: RwLock::new(HashMap::new()),
        }
    }

    fn get<T: Clone + 'static>(&self) -> ErasedChannel {
        let type_id = TypeId::of::<T>();
        if let Some(channel) = self.channels.read().get(&type_id) {
            channel.clone::<T>().unwrap()
        } else {
            let mut channels = self.channels.write();
            channels
                .entry(type_id)
                .or_insert_with(|| ErasedChannel::new::<T>(self.capacity))
                .clone::<T>()
                .unwrap()
        }
    }
}

#[derive(Debug)]
pub struct Events {
    shared: Arc<EventsShared>,
    channels: HashMap<TypeId, ErasedChannel>,
}

impl Clone for Events {
    fn clone(&self) -> Self {
        Events {
            shared: self.shared.clone(),
            channels: HashMap::new(),
        }
    }
}

impl Events {
    pub fn new_with_capacity(capacity: usize) -> Self {
        Events {
            shared: Arc::new(EventsShared::new(capacity)),
            channels: HashMap::new(),
        }
    }

    pub fn send<T: Clone + 'static>(&mut self, item: T) -> usize {
        self.get::<T>().send(item).unwrap()
    }

    pub fn subscribe<T: Clone + 'static>(&mut self) -> broadcast::Receiver<T> {
        self.get::<T>().subscribe().unwrap()
    }

    pub fn typed_sender<T: Clone + 'static>(&mut self) -> broadcast::Sender<T> {
        self.get::<T>().sender().unwrap().clone()
    }

    fn get<T: Clone + 'static>(&mut self) -> &ErasedChannel {
        let type_id = TypeId::of::<T>();
        self.channels
            .entry(type_id)
            .or_insert_with(|| self.shared.get::<T>())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
}
