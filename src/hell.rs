use std::collections::{HashMap};
use crate::{Gate, Error};
use tokio::{
    sync::{
        mpsc::{self, UnboundedSender}
    },
    task::JoinHandle
};

pub(crate) use self::mini_hell::MiniHell;
mod mini_hell;
#[cfg(feature = "ws")]
pub(crate) use self::mini_ws_hell::MiniWSHell;
#[cfg(feature = "ws")]
mod mini_ws_hell;

pub(crate) use self::hell_instruction::{HellInstruction};
mod hell_instruction;

pub(crate) use self::mini_hell_instruction::{MiniHellInstruction};
mod mini_hell_instruction;

/// ## Hell structure
///
/// This is equivalent to a normal actor framework system/runtime. A `Hell` instance will dispatch messages and coordinate interaction between actors.
pub struct Hell {
    // Demon counter, to asign a unique address to each demon
    counter: usize,
    // Communication channels with demons.
    demons: HashMap<usize, UnboundedSender<MiniHellInstruction>>
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
        Hell {
            counter: 0,
            demons: HashMap::new(),
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
        // Message communication for the gate
        let (hell_channel, mut instructions) = mpsc::unbounded_channel();
        
        let gate = Gate::new(hell_channel);
        let gate_clone = gate.clone();

        let jh = tokio::spawn(async move {
            #[cfg(feature = "internal_log")]
            log::info!("Hell starts to burn \u{1f525}");
            loop {
                tokio::select! {
                    value = instructions.recv() => if let Some(instruction) = value {
                        match instruction {
                            HellInstruction::CreateAddress{tx} => {
                                let current_counter = self.counter;
                                if tx.send(current_counter).is_ok() {
                                    self.counter += 1;
                                } else {
                                    #[cfg(feature = "internal_log")]
                                    log::warn!("Could not create a new address.");
                                }
                            },
                            HellInstruction::RegisterDemon{address, channel, tx} => {
                                let sent = match self.demons.entry(address) {
                                    std::collections::hash_map::Entry::Occupied(_) => {
                                        #[cfg(feature = "internal_log")]
                                        log::debug!("Demon address {} is already taken", address);
                                        false
                                    },
                                    std::collections::hash_map::Entry::Vacant(v) => {
                                        #[cfg(feature = "internal_log")]
                                        log::debug!("Registering new demon with address {}", address);
                                        v.insert(channel);
                                        true
                                    }
                                };

                                if tx.send(sent).is_err() {
                                    #[cfg(feature = "internal_log")]
                                    log::warn!("Dangling demon, as no answer could be returned.");
                                    self.demons.remove(&address);
                                }
                            },
                            HellInstruction::Message{tx, address, input} => {
                                if let Some(demon) = self.demons.get_mut(&address) {
                                    if demon.send(MiniHellInstruction::Message(tx, input)).is_err() {
                                        log::debug!("Message could not be delivered to demon {}", address);
                                    };
                                } else {
                                    if tx.send(Err(Error::InvalidLocation)).is_err() {
                                        log::debug!("Delivery failure notification could not arrive to demon {} caller", address);
                                    };
                                }
                            },
                            HellInstruction::RemoveDemon{address, tx} => {
                                // We simply drop the channel, so the mini hell will close automatically
                                let removed = self.demons.remove(&address);
                                if let Some(channel) = removed {
                                    if channel.send(MiniHellInstruction::Shutdown).is_err() {
                                        log::warn!("Could not notify demon minihell for demon removal");
                                        if tx.send(false).is_err() {
                                            log::warn!("Could not notify demon removal failure");
                                        }
                                    }
                                } else {
                                    #[cfg(feature = "internal_log")]
                                    log::debug!("The removal address did not exist");
                                    if tx.send(true).is_err() {
                                        log::warn!("Could not notify demon removal");
                                    }
                                }
                            }
                        }
                    } else {
                        #[cfg(feature = "internal_log")]
                        log::info!("All portals have been dropped, hell goes cold \u{1f9ca}");
                        break;
                    }
                }
            }
        });
        Ok((gate_clone, jh))
    }
}