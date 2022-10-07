use std::{
    any::{type_name, Any, TypeId},
    collections::HashMap,
    sync::Arc,
};
use tokio::sync::{broadcast, RwLock};

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

    async fn send<T: 'static>(&self, item: T) -> Result<usize, ErasedTypeError> {
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

    async fn get<T: Clone + 'static>(&self) -> ErasedChannel {
        let type_id = TypeId::of::<T>();
        if let Some(channel) = self.channels.read().await.get(&type_id) {
            channel.clone::<T>().unwrap()
        } else {
            let mut channels = self.channels.write().await;
            let channel = ErasedChannel::new::<T>(self.capacity);
            channels.insert(type_id, channel.clone::<T>().unwrap());
            channel
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

    pub async fn send<T: Clone + 'static>(&mut self, item: T) -> usize {
        self.get::<T>().await.send(item).await.unwrap()
    }

    pub async fn subscribe<T: Clone + 'static>(&mut self) -> broadcast::Receiver<T> {
        self.get::<T>().await.subscribe().unwrap()
    }

    async fn get<T: Clone + 'static>(&mut self) -> &ErasedChannel {
        let type_id = TypeId::of::<T>();
        if !self.channels.contains_key(&type_id) {
            let channel = self.shared.get::<T>().await;
            self.channels.insert(type_id, channel);
        }
        self.channels.get(&type_id).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
}
