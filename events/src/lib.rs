use parking_lot::RwLock;
use std::{
    any::{type_name, Any, TypeId},
    collections::HashMap,
    sync::Arc,
};
use tokio::sync::broadcast;

#[derive(Debug)]
pub struct Sender {
    shared: Arc<EventsShared>,
    channels: HashMap<TypeId, ErasedChannel>,
}

impl Clone for Sender {
    fn clone(&self) -> Self {
        Sender::new(self.shared.clone())
    }
}

impl Sender {
    fn new(shared: Arc<EventsShared>) -> Self {
        Sender {
            shared,
            channels: HashMap::new(),
        }
    }

    pub fn send<T: Clone + Send + 'static>(&mut self, item: T) -> usize {
        self.get::<T>().send(item).unwrap()
    }

    pub fn typed_sender<T: Clone + Send + 'static>(&mut self) -> TypedSender<T> {
        self.get::<T>().sender().unwrap().clone()
    }

    fn get<T: Clone + Send + 'static>(&mut self) -> &ErasedChannel {
        let type_id = TypeId::of::<T>();
        self.channels
            .entry(type_id)
            .or_insert_with(|| self.shared.get::<T>())
    }
}

#[derive(Debug)]
pub struct TypedSender<T>(broadcast::Sender<T>);

impl<T> Clone for TypedSender<T> {
    fn clone(&self) -> Self {
        TypedSender(self.0.clone())
    }
}

impl<T> TypedSender<T> {
    pub fn send(&self, item: T) -> usize {
        self.0.send(item).unwrap_or(0)
    }

    fn subscribe(&self) -> Subscriber<T> {
        Subscriber(self.0.subscribe())
    }
}

#[derive(Debug, thiserror::Error)]
#[error("sender closed")]
pub struct SenderClosed;

#[derive(Debug)]
pub struct Subscriber<T>(broadcast::Receiver<T>);

impl<T: Clone> Subscriber<T> {
    pub async fn recv(&mut self) -> Result<T, SenderClosed> {
        loop {
            match self.0.recv().await {
                Ok(item) => break Ok(item),
                Err(broadcast::error::RecvError::Closed) => break Err(SenderClosed),
                Err(broadcast::error::RecvError::Lagged(lag)) => {
                    tracing::warn!(lag, "lagged {lag} messages, retrying")
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct Events(Arc<EventsShared>);

impl Events {
    pub fn new_with_capacity(capacity: usize) -> Self {
        Events(Arc::new(EventsShared::new(capacity)))
    }

    pub fn sender(&mut self) -> Sender {
        Sender::new(self.0.clone())
    }

    pub fn typed_sender<T: Clone + Send + 'static>(&mut self) -> TypedSender<T> {
        self.0.get::<T>().sender().unwrap().clone()
    }

    pub fn subscribe<T: Clone + Send + 'static>(&mut self) -> Subscriber<T> {
        self.0.get::<T>().subscribe().unwrap()
    }
}

#[macro_export]
#[doc(hidden)]
macro_rules! subscriptions_type {
    ($n:ident) => {
        $n
    };
    ($n:ident $t:ty) => {
        $t
    };
}

#[macro_export]
macro_rules! subscriptions {
    { $name:ident => $( $n:ident $(: $t:ty)? ),+ } => {
        #[derive(Debug)]
        #[allow(non_snake_case)]
        struct $name {
            sender: $crate::Sender,
            $(
                $n: $crate::Subscriber<$crate::subscriptions_type!($n $($t)?)>,
            )*
        }

        impl $name {
            fn subscribe(events: &mut $crate::Events) -> Self {
                Self {
                    sender: events.sender(),
                    $(
                        $n: events.subscribe::<$crate::subscriptions_type!($n $($t)?)>(),
                    )*
                }
            }

            fn send<T: Clone + Send + 'static>(&mut self, item: T) -> usize {
                self.sender.send(item)
            }
        }
    };
    { $($args:tt)* } => {
        $crate::subscriptions! { Subscriptions => $($args)* }
    };
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

    fn get<T: Clone + Send + 'static>(&self) -> ErasedChannel {
        let type_id = TypeId::of::<T>();
        let existing_channel = self
            .channels
            .read()
            .get(&type_id)
            .map(|c| c.clone::<T>().unwrap());
        if let Some(channel) = existing_channel {
            channel
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

#[derive(Debug, thiserror::Error)]
#[error("ErasedChannel was of type {actual} but was expected to be of type {expected}")]
struct ErasedTypeError {
    expected: &'static str,
    actual: &'static str,
}

#[derive(Debug)]
struct ErasedChannel {
    sender: Box<dyn Any + Send + Sync>,
    type_name: &'static str,
}

impl ErasedChannel {
    fn new<T: Clone + Send + 'static>(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel::<T>(capacity);
        ErasedChannel {
            sender: Box::new(TypedSender(sender)),
            type_name: type_name::<T>(),
        }
    }

    fn clone<T: Send + 'static>(&self) -> Result<Self, ErasedTypeError> {
        let sender = self.sender::<T>()?;
        Ok(ErasedChannel {
            sender: Box::new(sender.clone()),
            type_name: self.type_name,
        })
    }

    fn send<T: 'static>(&self, item: T) -> Result<usize, ErasedTypeError> {
        Ok(self.sender()?.send(item))
    }

    fn subscribe<T: 'static>(&self) -> Result<Subscriber<T>, ErasedTypeError> {
        let sender = self.sender()?;
        Ok(sender.subscribe())
    }

    fn sender<T: 'static>(&self) -> Result<&TypedSender<T>, ErasedTypeError> {
        self.sender
            .downcast_ref::<TypedSender<T>>()
            .ok_or_else(|| ErasedTypeError {
                expected: type_name::<T>(),
                actual: self.type_name,
            })
    }
}

#[cfg(test)]
mod tests {
    // TODO tests
}
