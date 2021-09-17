use crate::{AnyDemon, Error};
use std::any::Any;
use tokio::sync::{oneshot::Sender, mpsc::UnboundedReceiver};

/// Structure that holds a single demon, and asynchronously deals with the messages that this demon receives.
pub(crate) struct MiniHell {
    /// Demon contained inside this minihell instance
    demon: Box<dyn AnyDemon>,
    /// Channel where to reply, and value to be fed to the demon
    queue: UnboundedReceiver<(Sender<Result<Box<dyn Any + Send>, Error>>, Box<dyn Any + Send>)>
}

impl MiniHell {
    pub fn spawn(demon: Box<dyn AnyDemon>, queue: UnboundedReceiver<(Sender<Result<Box<dyn Any + Send>, Error>>, Box<dyn Any + Send>)>) {
        let mini_hell = MiniHell {
            demon,
            queue
        };
        tokio::spawn(async move {
            mini_hell.fire().await;
        });
    }

    async fn fire(mut self) {
        loop {
            if let Some((tx, message)) = self.queue.recv().await {
                let r = self.demon.handle_any(message).await;
                if tx.send(r).is_err() {
                    #[cfg(feature = "debug")]
                    log::warn!("[Demon] I worked the answer, but I could not send it back...");   
                }
            } else {
                #[cfg(feature = "debug")]
                log::info!("This demon is no longer needed, bye");
                break;
            }
        }
    }
}