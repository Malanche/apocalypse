use crate::{Error, Demon, Location, hell::MiniHellInstruction};
use std::any::Any;
use tokio::sync::{oneshot::Sender, mpsc::{self, UnboundedSender, UnboundedReceiver}};

/// Structure that holds a single demon, and asynchronously deals with the messages that this demon receives.
pub(crate) struct MiniHell<D> {
    /// Demon contained inside this minihell instance
    demon: D,
    /// Address of this demon
    location: Location<D>,
    /// Channel where instructions are sent to the minihell
    instructions: UnboundedReceiver<MiniHellInstruction>
}

impl<I: 'static + Send, O: 'static + Send, D: 'static + Demon<Input = I, Output = O>> MiniHell<D> {
    pub fn spawn(demon: D, location: Location<D>) -> UnboundedSender<MiniHellInstruction> {
        let (mailbox, instructions) = mpsc::unbounded_channel();
        let mini_hell = MiniHell {
            demon,
            location,
            instructions
        };
        tokio::spawn(async move {
            mini_hell.fire().await;
        });
        mailbox
    }

    async fn fire(mut self) {
        let (mailbox, mut messages) = mpsc::unbounded_channel::<(Sender<Result<Box<dyn Any + Send>, Error>>, Box<dyn Any + Send>)>();

        // We call the spawned function from this demon
        let other_loc = self.location.clone();
        self.demon.spawned(other_loc).await;

        loop {
            tokio::select! {
                res = messages.recv() => if let Some((tx, input)) = res {
                    if let Ok(input) = input.downcast::<I>() {
                        let output = self.demon.handle(*input).await;
                        if tx.send(Ok(Box::new(output))).is_err() {
                            #[cfg(feature = "internal_log")]
                            log::warn!("[Demon] I worked the answer, but I could not send it back...");   
                        }
                    } else {
                        if tx.send(Err(Error::WrongType)).is_err() {
                            #[cfg(feature = "internal_log")]
                            log::warn!("[Demon] I worked the answer, but I could not send it back...");   
                        }
                    }
                } else {
                    #[cfg(feature = "internal_log")]
                    log::info!("This demon is no longer needed, bye");
                    break;
                },
                res = self.instructions.recv() => match res {
                    Some(instruction) => match instruction {
                        MiniHellInstruction::Shutdown => break,
                        MiniHellInstruction::Message(result_mailbox, message) => {
                            if mailbox.send((result_mailbox, message)).is_err() {
                                #[cfg(feature = "internal_log")]
                                log::warn!("[Demon] Impossible error happened, could not process message...");   
                            }
                        }
                    },
                    None => {
                        #[cfg(feature = "internal_log")]
                        log::warn!("Instructions will no longer arrive to hosting hell");
                        break;
                    }
                }
            }
        }

        // We call the spawned function from this demon
        self.demon.vanquished().await;
    }
}