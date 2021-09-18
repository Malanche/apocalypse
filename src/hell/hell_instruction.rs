use crate::{Error, hell::MiniHellInstruction};
use tokio::sync::{oneshot::Sender, mpsc::UnboundedSender};
use std::any::Any;

/// Actions that can be performed with the hell instance
pub(crate) enum HellInstruction {
    /// Requests address reservation
    CreateAddress {
        tx: Sender<usize>
    },
    /// Requests demon registration
    RegisterDemon {
        address: usize,
        channel: UnboundedSender<MiniHellInstruction>,
        tx: Sender<Result<(), Error>>
    },
    /// Requests demon removal
    RemoveDemon {
        address: usize,
        tx: Sender<Result<(), Error>>
    },
    /// Requests for a message to be delivered to a demon
    Message {
        tx: Sender<Result<Box<dyn Any + Send>, Error>>,
        address: usize,
        input: Box<dyn Any + Send>
    }
}