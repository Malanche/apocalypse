use crate::{Demon, Error};
use std::any::Any;
use cataclysm_ws::WebSocketReader;

/// AnyWSDemon trait
///
/// You should not have to implement or deal at all with this trait, I just document it so I don't forget what it is for.
/// The AnyWSDemon trait is a helper trait to emulate dynamic typing so that each actor can receive their own set of messages, and also can pass their own response (in contrast to having all actors receiving and sending the exact same types). Here, websocket handling is also included.
#[async_trait::async_trait]
pub(crate) trait AnyWSDemon: Send + WebSocketReader {
    /// Auxiliar wrapper function around handle
    async fn handle_any(&mut self, input: Box<dyn Any + Send>) -> Result<Box<dyn Any + Send>, Error>;
}

/// Implementation of the trait for basically every demon, under the assumption that Input and Output are 'static + Send.
#[async_trait::async_trait]
impl<I: 'static + Send, O: 'static + Send, E: Demon<Input = I, Output = O> + Send + WebSocketReader> AnyWSDemon for E {
    async fn handle_any(&mut self, input: Box<dyn Any + Send>) -> Result<Box<dyn Any + Send>, Error> {
        if let Ok(input) = input.downcast::<I>() {
            Ok(Box::new(self.handle(*input).await))
        } else {
            Err(Error::WrongType)
        }
    }
}