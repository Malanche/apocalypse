use std::any::Any;
use crate::{Error};
use tokio::sync::oneshot::Sender;

/// Message passing for the thread runner of each demon
pub(crate) enum MiniHellInstruction {
    /// Requests a graceful shutdown
    Shutdown(Sender<()>),
    /// Delivers a message to the demon
    Message(Sender<Result<Box<dyn Any + Send>, Error>>, Box<dyn Any + Send>)
}