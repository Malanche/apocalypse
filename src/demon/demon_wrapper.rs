use crate::{AnyDemon};
#[cfg(feature = "ws")]
use crate::AnyWSDemon;
#[cfg(feature = "ws")]
use tokio::net::tcp::OwnedReadHalf;

pub(crate) enum DemonWrapper {
    Demon(Box<dyn AnyDemon>),
    #[cfg(feature = "ws")]
    WSDemon(Box<dyn AnyWSDemon>, OwnedReadHalf)
}