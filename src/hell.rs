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
                                    #[cfg(feature = "internal_log")]
                                    log::debug!("[Hell] reserved address {}", current_counter);
                                    self.counter += 1;
                                } else {
                                    #[cfg(feature = "internal_log")]
                                    log::debug!("[Hell] failed to notify address {} reservation", current_counter);
                                }
                            },
                            HellInstruction::RegisterDemon{address, channel, tx} => {
                                let added = match self.demons.entry(address) {
                                    std::collections::hash_map::Entry::Occupied(_) => {
                                        #[cfg(feature = "internal_log")]
                                        log::debug!("[Hell] demon address {} is already taken", address);
                                        Err(Error::OccupiedAddress)
                                    },
                                    std::collections::hash_map::Entry::Vacant(v) => {
                                        #[cfg(feature = "internal_log")]
                                        log::debug!("[Hell] registering new demon with address {}", address);
                                        v.insert(channel);
                                        Ok(())
                                    }
                                };

                                if tx.send(added).is_err() {
                                    #[cfg(feature = "internal_log")]
                                    log::debug!("[Hell] dangling demon with address {}, as it could not be notified that it was registered. removing.", address);
                                    self.demons.remove(&address);
                                }
                            },
                            HellInstruction::Message{tx, address, input} => {
                                if let Some(demon) = self.demons.get_mut(&address) {
                                    if demon.send(MiniHellInstruction::Message(tx, input)).is_err() {
                                        #[cfg(feature = "internal_log")]
                                        log::debug!("[Hell] message could not be delivered to demon {}", address);
                                    };
                                } else {
                                    if tx.send(Err(Error::InvalidLocation)).is_err() {
                                        #[cfg(feature = "internal_log")]
                                        log::debug!("[Hell] invalid location message for address {} could not be delivered back", address);
                                    };
                                }
                            },
                            HellInstruction::RemoveDemon{address, tx} => {
                                // We simply drop the channel, so the mini hell will close automatically
                                let removed = self.demons.remove(&address);
                                if let Some(channel) = removed {
                                    if channel.send(MiniHellInstruction::Shutdown).is_err() {
                                        #[cfg(feature = "internal_log")]
                                        log::debug!("[Hell] could not notify demon thread the requested demon at address {} removal", address);
                                        if tx.send(Err(Error::DemonCommunication)).is_err() {
                                            #[cfg(feature = "internal_log")]
                                            log::debug!("[Hell] could not notify demon at address {} removal failure", address);
                                        }
                                    } else {
                                        if tx.send(Ok(())).is_err() {
                                            #[cfg(feature = "internal_log")]
                                            log::debug!("[Hell] could not notify back demon at address {} removal", address);
                                        }
                                    }
                                } else {
                                    #[cfg(feature = "internal_log")]
                                    log::debug!("[Hell] demon with address {} was not found", address);
                                    if tx.send(Err(Error::InvalidLocation)).is_err() {
                                        #[cfg(feature = "internal_log")]
                                        log::debug!("[Hell] could not notify that demon with address {} was not found", address);
                                    }
                                }
                            }
                        }
                    } else {
                        #[cfg(feature = "internal_log")]
                        log::info!("[Hell] all gates to hell have been dropped");
                        break;
                    }
                }
            }
            #[cfg(feature = "internal_log")]
            log::info!("[Hell] Hell goes cold \u{1f9ca}");
        });
        Ok((gate_clone, jh))
    }
}