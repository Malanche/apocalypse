use crate::{Error};
use tokio::sync::oneshot::Sender;
use std::any::Any;

pub enum Action {
    Invoke {
        tx: Sender<Result<Box<dyn Any + Send>, Error>>,
        address: usize,
        input: Box<dyn Any + Send>
    },
    Kill {
        address: usize
    }
}