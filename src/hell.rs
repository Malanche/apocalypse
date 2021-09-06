use std::collections::{HashMap};
use crate::{AnyDemon, Gate, Error};
use tokio::{
    sync::{
        mpsc::{self, UnboundedReceiver, UnboundedSender},
        oneshot::{Sender}
    },
    task::JoinHandle
};
use futures::{select_biased, future::FutureExt};
use std::any::Any;

use self::mini_hell::MiniHell;
mod mini_hell;

pub(crate) use self::action::{Action};
mod action;

/// ## Hell structure
///
/// This is equivalent to a normal actor framework system/runtime. A `Hell` instance will dispatch messages and coordinate interaction between actors.
pub struct Hell {
    // Demon counter, to asign a unique address to each demon
    counter: usize,
    // Communication channels with demons
    demons: HashMap<usize, UnboundedSender<(Sender<Result<Box<dyn Any + Send>, Error>>, Box<dyn Any + Send>)>>,
    // Current gate to current hell. Invalid once hell starts loose
    gate: Gate,
    // Source queue of messages
    message_source: UnboundedReceiver<Action>,
    // Demon queue, where new demons arrive
    demon_source: UnboundedReceiver<(Sender<usize>, Box<dyn AnyDemon>)>
}

impl Hell {
    /// Creates a new hell instance
    ///
    /// ```rust
    /// # use apocalypse::{Hell};
    /// let hell = Hell::new();
    /// // Now we can span demons!
    /// ```
    pub fn new() -> Hell {
        // Message communication for portals
        let (message_sink, message_source) = mpsc::unbounded_channel();

        // Message communication for demons
        let (demon_sink, demon_source) = mpsc::unbounded_channel();

        let gate = Gate::new(message_sink, demon_sink);

        Hell {
            counter: 0,
            demons: HashMap::new(),
            gate,
            message_source,
            demon_source
        }
    }

    /// Starts the actor system
    ///
    /// This method returns both a Gate, and a JoinHandle.
    ///
    /// ```rust,no_run
    /// use apocalypse::{Hell};
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let hell = Hell::new();
    ///     let (gate, join_handle) = hell.fire().await.unwrap();
    ///     // Do stuff with the gate
    ///     // ...
    ///     // Finally, await the actor's system execution
    ///     join_handle.await.unwrap();
    /// }
    /// ```
    pub async fn fire(mut self) -> Result<(Gate, JoinHandle<()>), Error>{
        let (tempm, _) = mpsc::unbounded_channel();
        let (tempd, _) = mpsc::unbounded_channel();
        let gate = std::mem::replace(&mut self.gate, Gate::new(tempm, tempd));

        let jh = tokio::spawn(async move {
            #[cfg(feature = "debug")]
            log::info!("Hell starts to burn \u{1f525}");
            loop {
                select_biased! {
                    value = self.message_source.recv().fuse() => {
                        if let Some(action) = value {
                            match action {
                                Action::Invoke{tx, address, input} => {
                                    if let Some(demon) = self.demons.get_mut(&address) {
                                        if demon.send((tx, input)).is_err() {
                                            log::warn!("Message could not be delivered to demon {}", address);
                                        };
                                    } else {
                                        if tx.send(Err(Error::InvalidLocation)).is_err() {
                                            log::warn!("Message could not be delivered to demon {}", address);
                                        };
                                    }
                                },
                                Action::Kill{address} => {
                                    // We simply drop the channel, so the mini hell will close automatically
                                    if self.demons.remove(&address).is_none() {
                                        #[cfg(feature = "debug")]
                                        log::warn!("Trying to remove non-existent demon...");
                                    }
                                }
                            }
                        } else {
                            #[cfg(feature = "debug")]
                            log::info!("All portals have been dropped, hell goes cold \u{1f9ca}");
                            break;
                        }
                    },
                    value = self.demon_source.recv().fuse() => {
                        if let Some((tx, demon)) = value {
                            let current_counter = self.counter;
                            if tx.send(current_counter).is_ok() {
                                #[cfg(feature = "debug")]
                                log::info!("Spawning new demon with address {}", current_counter);
                                
                                self.counter += 1;
                                let (mhtx, mhrx) = mpsc::unbounded_channel();
                                MiniHell::spawn(demon, mhrx);
                                self.demons.insert(current_counter, mhtx);
                            } else {
                                #[cfg(feature = "debug")]
                                log::warn!("Dangling demon with address {}, removing from hell.", current_counter);
                            }
                        } else {
                            // As both sinks travel together in a portal, this block either
                            // 1. Is never called
                            // 2. Is called before the messagae_source returns None, which stops the loop
                        }
                    }
                }
            }
        });
        Ok((gate, jh))
    }
}