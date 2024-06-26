use crate::{Error, Demon, Location, hell::{MiniHellInstruction, DemonChannels}};
use std::any::Any;
use tokio::sync::{oneshot::{Sender}, mpsc::{self, UnboundedReceiver}};

/// Structure that holds a single demon, and asynchronously deals with the messages that this demon receives.
pub(crate) struct MiniHell<D> {
    /// Demon contained inside this minihell instance
    demon: D,
    /// Address of this demon
    location: Location<D>,
    /// Channel where instructions are sent to the minihell
    instructions: UnboundedReceiver<MiniHellInstruction>,
    /// Killswitch endpoint
    killswitch: UnboundedReceiver<Sender<()>>
}

impl<I: 'static + Send, O: 'static + Send, D: 'static + Demon<Input = I, Output = O>> MiniHell<D> {
    pub fn spawn(demon: D, location: Location<D>) -> DemonChannels {
        // Main instruction channel
        let (mailbox, instructions) = mpsc::unbounded_channel();
        // Killswitch channel
        let (killswitch_tx, killswitch) = mpsc::unbounded_channel();
        let mini_hell = MiniHell {
            demon,
            location,
            instructions,
            killswitch
        };
        tokio::spawn(async move {
            mini_hell.ignite().await;
        });

        DemonChannels {
            instructions: mailbox,
            killswitch: killswitch_tx
        }
    }

    async fn ignite(mut self) {
        #[cfg(feature = "full_log")]
        log::trace!("[{}] demon thread starting", self.demon.id());
        let (mailbox, mut messages) = mpsc::unbounded_channel::<(Sender<Result<Box<dyn Any + Send>, Error>>, Box<dyn Any + Send>)>();

        // We call the spawned function from this demon
        let other_loc = self.location.clone();
        #[cfg(feature = "full_log")]
        log::trace!("[{}] calling spawn function", self.demon.id());
        self.demon.spawned(other_loc).await;
        #[cfg(feature = "full_log")]
        log::trace!("[{}] spawn function called", self.demon.id());

        let (mut vanquish_mailbox, killswitched) = loop {
            tokio::select! {
                res = self.killswitch.recv() => if let Some(vanquish_mailbox) = res {
                    #[cfg(feature = "full_log")]
                    log::trace!("[{}] killswitch message received, forced demon shutdown", self.demon.id());
                    break (Some(vanquish_mailbox), true);
                } else {
                    #[cfg(feature = "full_log")]
                    log::trace!("[{}] all incoming killswitch channels closed (impossible)", self.demon.id());
                    break (None, true);
                },
                res = messages.recv() => if let Some((tx, input)) = res {
                    if let Ok(input) = input.downcast::<I>() {
                        #[cfg(feature = "full_log")]
                        log::trace!("[{}] calling handle function", self.demon.id());
                        let output = tokio::select!{
                            output = self.demon.handle(*input) => {
                                #[cfg(feature = "full_log")]
                                log::trace!("[{}] handle function called", self.demon.id());
                                output
                            },
                            res = self.killswitch.recv() => if let Some(vanquish_mailbox) = res {
                                #[cfg(feature = "full_log")]
                                log::trace!("[{}] killswitch signal received, aborting current handle execution!", self.demon.id());
                                break (Some(vanquish_mailbox), true);
                            } else {
                                #[cfg(feature = "full_log")]
                                log::trace!("[{}] all incoming killswitch channels closed (impossible), aborting current handle execution", self.demon.id());
                                break (None, true);
                            }
                        };
                        #[cfg(feature = "full_log")]
                        log::trace!("[{}] demon processed message!", self.demon.id());
                        if tx.send(Ok(Box::new(output))).is_err() {
                            #[cfg(feature = "full_log")]
                            log::error!("[{}] demon processed message could not be sent back", self.demon.id());   
                        }
                    } else {
                        if tx.send(Err(Error::WrongType)).is_err() {
                            #[cfg(feature = "full_log")]
                            log::error!("[{}] somehow, demon received wrong message type", self.demon.id());   
                        }
                    }
                } else {
                    #[cfg(feature = "full_log")]
                    log::trace!("[{}] all incoming channels closed (impossible)", self.demon.id());
                    break (None, false);
                },
                res = self.instructions.recv() => match res {
                    Some(instruction) => match instruction {
                        MiniHellInstruction::Shutdown(vanquish_mailbox) => {
                            #[cfg(feature = "full_log")]
                            log::trace!("[{}] shutdown signal received", self.demon.id());
                            break (Some(vanquish_mailbox), false);
                        },
                        MiniHellInstruction::Message(result_mailbox, message) => {
                            #[cfg(feature = "full_log")]
                            log::trace!("[{}] received instruction, adding to the processing queue", self.demon.id());
                            if mailbox.send((result_mailbox, message)).is_err() {
                                #[cfg(feature = "full_log")]
                                log::warn!("[{}] impossible error happened, could not send back message to itself!", self.demon.id());
                            }
                        }
                    },
                    None => {
                        #[cfg(feature = "full_log")]
                        log::info!("[{}] all channels to this demon are now closed", self.demon.id());
                        break (None, false);
                    }
                }
            }
        };

        #[cfg(feature = "full_log")]
        let demon_id = self.demon.id();

        // We call the vanquished function from this demon
        #[cfg(feature = "full_log")]
        log::trace!("[{}] calling vanquish function", demon_id);
        if !killswitched {
            tokio::select!{
                res = self.killswitch.recv() => if let Some(vm) = res {
                    #[cfg(feature = "full_log")]
                    log::trace!("[{}] killswitch message received, canceling vanquished function", demon_id);
                    vanquish_mailbox = Some(vm);
                } else {
                    #[cfg(feature = "full_log")]
                    log::trace!("[{}] all incoming killswitch channels closed (impossible)", demon_id);
                },
                _ = self.demon.vanquished() => {
                    #[cfg(feature = "full_log")]
                    log::trace!("[{}] vanquish function called", demon_id);
                }
            }
        } else {
            #[cfg(feature = "full_log")]
            log::trace!("[{}] skipping vanquish function due to killswitch detection", demon_id);
        }

        if let Some(vanquish_mailbox) = vanquish_mailbox {
            if vanquish_mailbox.send(()).is_err() {
                #[cfg(feature = "full_log")]
                log::warn!("[{}] could not notify back hell about shutdown!", demon_id);   
            }
        } else {
            #[cfg(feature = "full_log")]
            log::trace!("[{}] it seems that there are no channels to anounce departure", demon_id);
        }
    }
}