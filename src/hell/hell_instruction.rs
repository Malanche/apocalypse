use crate::{Error, hell::MiniHellInstruction};
use tokio::sync::{oneshot::Sender, mpsc::UnboundedSender};
use std::any::Any;

/// Actions that can be performed with the hell instance
pub enum HellInstruction {
    CreateAddress {
        tx: Sender<usize>
    },
    RegisterDemon {
        address: usize,
        channel: UnboundedSender<MiniHellInstruction>,
        tx: Sender<bool>
    },
    RemoveDemon {
        address: usize,
        tx: Sender<bool>
    },
    Message {
        tx: Sender<Result<Box<dyn Any + Send>, Error>>,
        address: usize,
        input: Box<dyn Any + Send>
    }
}