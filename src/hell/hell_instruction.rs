use crate::{Error, hell::{DemonChannels, HellStats}};
use tokio::sync::{oneshot::Sender};
use std::any::Any;
use std::time::Duration;

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
        tx: Sender<Result<(), Error>>,
        /// Ignore flag, indicates if we should wait for the demon to be dead
        ignore: bool,
        /// Maximum time that we wait for the demon before dropping all messages and Futures
        force: Option<Option<Duration>>
    },
    /// Requests for a message to be delivered to a demon
    Message {
        tx: Sender<Result<Box<dyn Any + Send>, Error>>,
        /// Location for the message
        address: usize,
        /// Ignore flag, indicates if we should wait for the demon to reply or not
        ignore: bool,
        input: Box<dyn Any + Send>
    },
    /// Requests the stats structure
    Stats {
        tx: Sender<HellStats>
    },
    /// Asks for termination
    Extinguish {
        tx: Sender<Result<(), Error>>,
        timeout: Option<Option<Duration>>
    }
}