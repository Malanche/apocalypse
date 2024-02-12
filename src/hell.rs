use std::{
    collections::{HashMap},
    time::Duration
};
use crate::{Gate, Error};
use tokio::{
    sync::{
        oneshot::{self},
        mpsc::{self}
    },
    task::JoinHandle
};
use chrono::{DateTime, Utc};

pub(crate) use self::mini_hell::MiniHell;
mod mini_hell;
pub(crate) use self::multiple_mini_hell::MultipleMiniHell;
mod multiple_mini_hell;
#[cfg(feature = "ws")]
pub(crate) use self::mini_ws_hell::MiniWSHell;
#[cfg(feature = "ws")]
mod mini_ws_hell;

pub(crate) use self::demon_channels::{DemonChannels};
mod demon_channels;

pub(crate) use self::hell_stats::{HellStats};
mod hell_stats;

pub(crate) use self::hell_instruction::{HellInstruction, Channel};
mod hell_instruction;

pub(crate) use self::mini_hell_instruction::{MiniHellInstruction};
mod mini_hell_instruction;

/// Builder helper for a Hell instance
pub struct HellBuilder {
    /// Timeout before shutdown of a demon
    timeout: Option<Duration>
}

impl HellBuilder {
    /// Generates a new instance of a hell builder
    ///
    /// ```rust
    /// # use apocalypse::{HellBuilder};
    /// # fn main() {
    /// let hell = HellBuilder::new();
    /// // Change params of the hell instance
    /// # }
    /// ```
    pub fn new() -> HellBuilder {
        HellBuilder {
            timeout: None
        }
    }

    /// Sets a timeout for the vanquish method to be executed
    ///
    /// ```rust
    /// use apocalypse::{HellBuilder};
    /// use std::time::Duration;
    ///
    /// # fn main() {
    /// let hell = HellBuilder::new().timeout(Duration::from_secs(5));
    /// // further modify this hell instance
    /// # }
    /// ```
    pub fn timeout(mut self, timeout: std::time::Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Generates the hell instance from the builder params
    ///
    /// ```rust
    /// # use apocalypse::{HellBuilder};
    /// # fn main() {
    /// let hell = HellBuilder::new().build();
    /// # }
    /// ```
    pub fn build(self) -> Hell {
        Hell {
            counter: 0,
            zombie_counter: 0,
            successful_messages: 0,
            failed_messages: 0,
            demons: HashMap::new(),
            timeout: self.timeout,
            ignition_time: Utc::now()
        }
    }
}

/// ## Hell structure
///
/// This is equivalent to a normal actor framework system/runtime. A `Hell` instance will dispatch messages and coordinate interaction between actors.
pub struct Hell {
    /// Demon counter, to asign a unique address to each demon
    counter: usize,
    /// Amount of messages delivered to demons
    successful_messages: usize,
    /// Amount of messages delivered to demons
    failed_messages: usize,
    /// Zombie counter
    zombie_counter: usize,
    /// Communication channels with demons.
    demons: HashMap<usize, DemonChannels>,
    /// Maximum wait time for killswitch calls
    timeout: Option<Duration>,
    /// Time that hell has been active
    ignition_time: DateTime<Utc>
}

impl Hell {
    /// Creates a new hell instance with default parameters
    ///
    /// In this case, a timeout is not set, and vanquish calls are executed until the demon gracefully shuts down
    ///
    /// ```rust
    /// # use apocalypse::{Hell};
    /// let hell = Hell::new();
    /// // Now we can spawn demons!
    /// ```
    pub fn new() -> Hell {
        Hell {
            counter: 0,
            zombie_counter: 0,
            successful_messages: 0,
            failed_messages: 0,
            demons: HashMap::new(),
            timeout: None,
            ignition_time: Utc::now()
        }
    }

    /// Creates a new [HellBuilder](HellBuilder)
    ///
    /// ```rust
    /// # use apocalypse::{Hell};
    /// let hell = Hell::builder().timeout(std::time::Duration::from_secs(5)).build();
    /// // Now we can spawn demons!
    /// ```
    pub fn builder() -> HellBuilder {
        HellBuilder::new()
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
    ///     let (gate, join_handle) = hell.ignite().await.unwrap();
    ///     // Do stuff with the gate
    ///     // ...
    ///     // Finally, await the actor's system execution
    ///     join_handle.await.unwrap();
    /// }
    /// ```
    pub async fn ignite(mut self) -> Result<(Gate, JoinHandle<()>), Error>{
        // ignition time update
        self.ignition_time = Utc::now();

        // Message communication for the gate
        let (hell_channel, mut instructions) = mpsc::unbounded_channel();
        
        let gate = Gate::new(hell_channel);
        let gate_clone = gate.clone();

        let jh = tokio::spawn(async move {
            #[cfg(feature = "full_log")]
            log::info!("Broker starts \u{1f525}");

            // We need another channel, for zombie count removal
            let (zombie_tx, mut zombie_rx) = mpsc::unbounded_channel();

            let clean = loop {
                tokio::select! {
                    value = instructions.recv() => if let Some(instruction) = value {
                        match instruction {
                            HellInstruction::CreateAddress{tx} => {
                                let current_counter = self.counter;
                                if tx.send(current_counter).is_ok() {
                                    #[cfg(feature = "full_log")]
                                    log::debug!("[Hell] reserved address {}", current_counter);
                                    self.counter += 1;
                                } else {
                                    #[cfg(feature = "full_log")]
                                    log::debug!("[Hell] failed to notify address {} reservation", current_counter);
                                }
                            },
                            HellInstruction::RegisterDemon{address, demon_channels, tx} => {
                                let added = match self.demons.entry(address) {
                                    std::collections::hash_map::Entry::Occupied(_) => {
                                        #[cfg(feature = "full_log")]
                                        log::debug!("[Hell] demon address {} is already taken", address);
                                        Err(Error::OccupiedAddress)
                                    },
                                    std::collections::hash_map::Entry::Vacant(v) => {
                                        #[cfg(feature = "full_log")]
                                        log::debug!("[Hell] registering new demon with address {}", address);
                                        v.insert(demon_channels);
                                        Ok(())
                                    }
                                };

                                if tx.send(added).is_err() {
                                    #[cfg(feature = "full_log")]
                                    log::debug!("[Hell] dangling demon with address {}, as it could not be notified that it was registered. removing.", address);
                                    self.demons.remove(&address);
                                }
                            },
                            HellInstruction::Message{tx, address, input} => {
                                if let Some(demon_channels) = self.demons.get_mut(&address) {
                                    if demon_channels.instructions.send(MiniHellInstruction::Message(tx, input)).is_err() {
                                        self.failed_messages += 1;
                                        #[cfg(feature = "full_log")]
                                        log::debug!("[Hell] message could not be delivered to demon {}", address);
                                    } else {
                                        self.successful_messages += 1;
                                    };
                                } else {
                                    if tx.send(Err(Error::InvalidLocation)).is_err() {
                                        #[cfg(feature = "full_log")]
                                        log::debug!("[Hell] invalid location message for address {} could not be delivered back", address);
                                    };
                                }
                            },
                            HellInstruction::RemoveDemon{address, tx, ignore, force} => {
                                // We simply drop the channel, so the mini hell will close automatically
                                let removed = self.demons.remove(&address);
                                if let Some(demon_channels) = removed {
                                    // This channel will allow the zombie counter to be decreased, when necessary
                                    let (demon_tx, demon_rx) = oneshot::channel();
                                    let (killswitch_tx, killswitch) = oneshot::channel();

                                    // force timeout has the prefference
                                    let timeout = match force {
                                        Some(v) => v,
                                        None => self.timeout
                                    };

                                    if let Some(timeout) = timeout {
                                        #[cfg(feature = "full_log")]
                                        log::debug!("[Hell] killswitch trigger requested in {}ms", timeout.as_millis());
                                        // We send the killswitch with a timeout
                                        let demon_channel_killswitch = demon_channels.killswitch;
                                        tokio::spawn(async move {
                                            tokio::time::sleep(timeout).await;
                                            // We ignore the killswitch send, because maybe the demon_channel is already obsolete
                                            let _ = demon_channel_killswitch.send(killswitch_tx);
                                        });
                                    } else {
                                        #[cfg(feature = "full_log")]
                                        log::debug!("[Hell] no timeout was set for this vanquish call");
                                    }

                                    if demon_channels.instructions.send(MiniHellInstruction::Shutdown(demon_tx)).is_err() {
                                        #[cfg(feature = "full_log")]
                                        log::debug!("[Hell] could not notify demon thread the requested demon at address {} removal", address);
                                        if tx.send(Err(Error::DemonCommunication)).is_err() {
                                            #[cfg(feature = "full_log")]
                                            log::debug!("[Hell] could not notify demon at address {} removal failure", address);
                                        }
                                    } else {
                                        let _address_copy = address.clone();
                                        let zombie_tx_clone = zombie_tx.clone();
                                        let waiter = async move {
                                            tokio::select! {
                                                res = demon_rx => {
                                                    if res.is_err() {
                                                        #[cfg(feature = "full_log")]
                                                        log::debug!("[Hell] could not wait for demon to be vanquished, {}", _address_copy);
                                                    } else {
                                                        #[cfg(feature = "full_log")]
                                                        log::debug!("[Hell] gracefull vanquish, {}", _address_copy);
                                                    }
                                                },
                                                res = killswitch => {
                                                    if res.is_err() {
                                                        #[cfg(feature = "full_log")]
                                                        log::debug!("[Hell] could not wait for demon to be killswitch vanquished, {}", _address_copy);
                                                    } else {
                                                        #[cfg(feature = "full_log")]
                                                        log::debug!("[Hell] killswitch vanquish requested, sending to address {}", _address_copy);
                                                    }
                                                }
                                            };

                                            if ignore {
                                                if zombie_tx_clone.send(()).is_err() {
                                                    #[cfg(feature = "full_log")]
                                                    log::debug!("[Hell] demon zombie counter message decrease could not be sent");
                                                }
                                            }
                                        };
                                        // if the message should be ignored, we need to move it to a different thread
                                        if ignore {
                                            self.zombie_counter += 1;
                                            tokio::spawn(waiter);
                                        } else {
                                            waiter.await;
                                        }

                                        if tx.send(Ok(())).is_err() {
                                            #[cfg(feature = "full_log")]
                                            log::debug!("[Hell] could not notify back demon at address {} removal", address);
                                        }
                                    }
                                } else {
                                    #[cfg(feature = "full_log")]
                                    log::debug!("[Hell] demon with address {} was not found", address);
                                    if tx.send(Err(Error::InvalidLocation)).is_err() {
                                        #[cfg(feature = "full_log")]
                                        log::debug!("[Hell] could not notify that demon with address {} was not found", address);
                                    }
                                }
                            },
                            HellInstruction::Stats{tx} => {
                                if tx.send(HellStats {
                                    spawned_demons: self.counter,
                                    active_demons: self.demons.len(),
                                    zombie_demons: 0,
                                    successful_messages: self.successful_messages,
                                    failed_messages: self.failed_messages,
                                    ignition_time: self.ignition_time.clone()
                                }).is_err() {
                                    #[cfg(feature = "full_log")]
                                    log::debug!("[Hell] could not return hell stats, channel closed");
                                }
                            },
                            HellInstruction::Extinguish{tx, wait} => {
                                break Some((tx, wait));
                            }
                        }
                    } else {
                        #[cfg(feature = "full_log")]
                        log::info!("[Hell] all gates to hell have been dropped");
                        break None;
                    },
                    value = zombie_rx.recv() => if value.is_some() {
                        self.zombie_counter -= 1;
                    } else {
                        log::error!("[Hell] impossible failure, channel was closed unexpectedly");
                        break None;
                    }
                }
            };

            if let Some((tx, wait)) = clean {
                for (_id, demon_channels) in self.demons {
                    let (tx, rx) = oneshot::channel();
                    if demon_channels.instructions.send(MiniHellInstruction::Shutdown(tx)).is_err() {
                        #[cfg(feature = "full_log")]
                        log::debug!("[Hell] could not notify demon thread the requested demon at address {} removal", _id);
                    }
                    if wait {
                        if rx.await.is_err() {
                            #[cfg(feature = "full_log")]
                            log::debug!("[Hell] could not wait for demon to be vanquished, {}", _id);
                        }
                    }
                }
                if tx.send(()).is_err() {
                    #[cfg(feature = "full_log")]
                    log::debug!("[Hell] could not notify gate about extintion");
                }
            }
            #[cfg(feature = "full_log")]
            log::info!("Broker stops \u{1f9ca}");
        });
        Ok((gate_clone, jh))
    }
}