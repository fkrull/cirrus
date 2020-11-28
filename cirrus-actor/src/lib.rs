use futures::channel::mpsc;

mod actor;
pub use actor::*;
mod messages;
pub use messages::*;

pub mod util;

#[derive(Debug, thiserror::Error)]
#[error("sending message failed")]
pub struct SendError(#[from] mpsc::SendError);
