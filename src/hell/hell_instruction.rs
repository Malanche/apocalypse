use crate::{Error, hell::{DemonChannels, HellStats}};
use tokio::sync::{oneshot::Sender};
use std::any::Any;
use std::time::Duration;

/// Wrapper around a channel to either be sync or async
pub(crate) enum Channel<T> {
    Sync(std::sync::mpsc::Sender<T>),
    Async(Sender<T>)
}

impl<T> Channel<T> {
    /// Tries to send a value through the channel
    pub fn send(self, value: T) -> Result<(), T> {
        match self {
            Channel::Sync(sender) => sender.send(value).map_err(|e| e.0),
            Channel::Async(sender) => sender.send(value)
        }
    }
}

/// Actions that can be performed with the hell instance
pub(crate) enum HellInstruction {
    /// Requests address reservation
    CreateAddress {
        tx: Sender<usize>
    },
    /// Requests demon registration
    RegisterDemon {
        address: usize,
        demon_channels: DemonChannels,
        tx: Sender<Result<(), Error>>
    },
    /// Requests demon removal
    RemoveDemon {
        address: usize,
        /// Optional wait channel
        tx: Channel<Result<(), Error>>,
        /// Ignore flag, indicates if we should wait for the demon to be dead
        ignore: bool,
        /// Maximum time that we wait for the demon before dropping all messages and Futures
        force: Option<Duration>
    },
    /// Requests for a message to be delivered to a demon
    Message {
        tx: Sender<Result<Box<dyn Any + Send>, Error>>,
        address: usize,
        input: Box<dyn Any + Send>
    },
    /// Requests the stats structure
    Stats {
        tx: Sender<HellStats>
    },
    /// Asks for termination
    Extinguish {
        tx: Sender<()>,
        wait: bool
    }
}