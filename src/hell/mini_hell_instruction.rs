use std::any::Any;
use crate::{Error};
use tokio::sync::oneshot::Sender;

pub enum MiniHellInstruction {
    Shutdown,
    Message(Sender<Result<Box<dyn Any + Send>, Error>>, Box<dyn Any + Send>)
}