use crate::{Error, Demon, Location, hell::MiniHellInstruction};
use std::any::Any;

use tokio::{
    sync::{oneshot::Sender, mpsc::{self, UnboundedSender, UnboundedReceiver}}
};
use cataclysm::ws::{WebSocketReader, WebSocketThread};

/// Structure that holds a single demon, and asynchronously deals with the messages that this demon receives.
pub(crate) struct MiniWSHell<D> {
    /// Demon contained inside this minihell instance
    demon: D,
    /// Address of this demon
    location: Location<D>,
    /// Channel where instructions are sent to the minihell
    instructions: UnboundedReceiver<MiniHellInstruction>,
    /// Read stream where ws messages arrive
    wsr: WebSocketReader
}

impl<I: 'static + Send, O: 'static + Send, D: 'static + Demon<Input = I, Output = O> + WebSocketThread> MiniWSHell<D> {
    pub(crate) fn spawn(demon: D, location: Location<D>, wsr: WebSocketReader) -> UnboundedSender<MiniHellInstruction> {
        let (mailbox, instructions) = mpsc::unbounded_channel();
        let mini_hell = MiniWSHell {
            demon,
            location,
            instructions,
            wsr
        };
        tokio::spawn(async move {
            mini_hell.fire().await;
        });
        mailbox
    }

    async fn fire(mut self) {
        #[cfg(feature = "full_log")]
        log::debug!("[{}] demon thread starting", self.demon.id());
        // Inner message passing
        let (mailbox, mut messages) = mpsc::unbounded_channel::<(Sender<Result<Box<dyn Any + Send>, Error>>, Box<dyn Any + Send>)>();

        // We call both opening callbacks, starting by the websockets one
        self.demon.on_open().await;
        let other_loc = self.location.clone();
        self.demon.spawned(other_loc).await;

        let notify = loop {
            tokio::select! {
                res = messages.recv() => if let Some((tx, input)) = res {
                    if let Ok(input) = input.downcast::<I>() {
                        #[cfg(feature = "full_log")]
                        log::debug!("[{}] demon ready to process message...", self.demon.id()); 
                        let output = self.demon.handle(*input).await;
                        #[cfg(feature = "full_log")]
                        log::debug!("[{}] demon processed message!", self.demon.id());
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
                    log::debug!("[{}] all incoming channels closed (impossible)", self.demon.id());
                    break None;
                },
                frame = self.wsr.try_read_frame() => match frame {
                    Ok(frame) => {
                        if frame.message.is_close() {
                            self.demon.on_close(true).await;
                            break None;
                        }

                        self.demon.on_message(frame.message).await;
                    },
                    Err(_e) => {
                        self.demon.on_close(false).await;
                        #[cfg(feature = "full_log")]
                        log::debug!("[{}] {}", self.demon.id(), _e);

                        break None;
                    }
                },
                res = self.instructions.recv() => match res {
                    Some(instruction) => match instruction {
                        MiniHellInstruction::Shutdown(tx) => {
                            #[cfg(feature = "full_log")]
                            log::debug!("[{}] shutdown signal received", self.demon.id());
                            break Some(tx);
                        },
                        MiniHellInstruction::Message(result_mailbox, message) => {
                            #[cfg(feature = "full_log")]
                            log::debug!("[{}] received instruction, adding to the processing queue", self.demon.id());
                            if mailbox.send((result_mailbox, message)).is_err() {
                                #[cfg(feature = "full_log")]
                                log::warn!("[{}] impossible error happened, could not send back message to itself!", self.demon.id());   
                            }
                        }
                    },
                    None => {
                        #[cfg(feature = "full_log")]
                        log::info!("[{}] all channels to this demon are now closed", self.demon.id());
                        break None;
                    }
                }
            }
        };

        // The same, in reverse order
        self.demon.vanquished(self.location).await;
        if let Some(vanquish_mailbox) = notify {
            if vanquish_mailbox.send(()).is_err() {
                #[cfg(feature = "full_log")]
                log::warn!("[{}] could not notify back hell about shutdown!", self.demon.id());   
            }
        }
        #[cfg(feature = "full_log")]
        log::debug!("[{}] demon thread finished", self.demon.id());
    }
}