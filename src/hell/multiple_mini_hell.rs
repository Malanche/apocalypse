use crate::{Error, Demon, Location, hell::{MiniHellInstruction, DemonChannels}};
use std::any::Any;
use std::collections::{VecDeque, HashMap};
use tokio::sync::{oneshot::{Sender}, mpsc::{self, UnboundedReceiver}};

/// Structure that holds a single demon, and asynchronously deals with the messages that this demon receives.
pub(crate) struct MultipleMiniHell<D> {
    /// Demon contained inside this minihell instance
    demons: VecDeque<(usize, D)>,
    /// Address of this demon
    location: Location<D>,
    /// Channel where instructions are sent to the minihell
    instructions: UnboundedReceiver<MiniHellInstruction>,
    /// Killswitch endpoint
    killswitch: UnboundedReceiver<Sender<()>>
}

impl<I: 'static + Send, O: 'static + Send, D: 'static + Demon<Input = I, Output = O>> MultipleMiniHell<D> {
    pub fn spawn<F: FnMut() -> D>(mut demon_factory: F, replicas: usize, location: Location<D>) -> Result<DemonChannels, Error> {
        // Main instruction channel
        let (mailbox, instructions) = mpsc::unbounded_channel();
        // Killswitch channel
        let (killswitch_tx, killswitch) = mpsc::unbounded_channel();

        let demons: VecDeque<(usize, D)> = (0..replicas).map(|idx| (idx, demon_factory())).collect();

        let multiple_mini_hell = MultipleMiniHell {
            demons,
            location,
            instructions,
            killswitch
        };

        tokio::spawn(async move {
            multiple_mini_hell.ignite().await;
        });

        Ok(DemonChannels {
            instructions: mailbox,
            killswitch: killswitch_tx
        })
    }

    async fn ignite(mut self) {
        #[cfg(feature = "full_log")]
        log::debug!("[{}] multiple demon thread starting", <D as Demon>::multiple_id());
        let (mailbox, mut messages) = mpsc::unbounded_channel::<(Sender<Result<Box<dyn Any + Send>, Error>>, Box<dyn Any + Send>)>();

        // Answers channel
        let (answers_tx, mut answers) = mpsc::unbounded_channel::<(usize, D)>();
        let mut requests: VecDeque<(
            Sender<Result<Box<dyn Any + Send>, Error>>,
            I
        )> = VecDeque::new();

        let mut handles: HashMap<usize, tokio::task::JoinHandle<()>> = HashMap::new();

        // We call the spawned function from this demon
        for (_, demon) in &mut self.demons {
            #[cfg(feature = "full_log")]
            log::debug!("[{}] calling spawn function", demon.id());
            demon.spawned(self.location.clone()).await;
            #[cfg(feature = "full_log")]
            log::debug!("[{}] spawn function called", demon.id());
        }

        let vanquish_mailbox = loop {
            tokio::select! {
                answer = answers.recv() => if let Some((idx, mut demon)) = answer {
                    // if we have pending requests, we pop them here
                    if let Some((tx, request)) = requests.pop_front() {
                        let answers_tx_clone = answers_tx.clone();
                        handles.insert(idx, tokio::spawn(async move {
                            #[cfg(feature = "full_log")]
                            log::debug!("[{}] calling handle function", demon.id());
                            let output = demon.handle(request).await;
                            #[cfg(feature = "full_log")]
                            log::debug!("[{}] handle function called", demon.id());
                            
                            // We first send the reply
                            if tx.send(Ok(Box::new(output))).is_err() {
                                #[cfg(feature = "full_log")]
                                log::error!("[{}] demon processed message could not be sent back", demon.id());
                            }

                            // Now the demon back
                            #[cfg(feature = "full_log")]
                            let demon_id = demon.id();
                            if answers_tx_clone.send((idx, demon)).is_err() {
                                #[cfg(feature = "full_log")]
                                log::error!("[{}] demon could not be sent back", demon_id);
                            }
                        }));
                    } else {
                        handles.remove(&idx);
                        self.demons.push_back((idx, demon));
                    }
                } else {
                    #[cfg(feature = "full_log")]
                    log::debug!("[{}] all incoming answer channels closed (impossible)", <D as Demon>::multiple_id());
                    break None;
                },
                res = self.killswitch.recv() => if let Some(vanquish_mailbox) = res {
                    #[cfg(feature = "full_log")]
                    log::debug!("[{}] killswitch message received, forced demon shutdown (aborting {} pending tasks)", <D as Demon>::multiple_id(), handles.len());
                    for handle in handles.into_iter().map(|v| v.1) {
                        handle.abort()
                    }
                    break Some(vanquish_mailbox);
                } else {
                    #[cfg(feature = "full_log")]
                    log::debug!("[{}] all incoming killswitch channels closed (impossible)", <D as Demon>::multiple_id());
                    break None;
                },
                res = messages.recv() => if let Some((tx, input)) = res {
                    if let Ok(input) = input.downcast::<I>() {
                        if let Some((idx, mut demon)) = self.demons.pop_front() {
                            #[cfg(feature = "full_log")]
                            log::debug!("[{}] available demon, sending to thread to process message. remaining demons: {}", demon.id(), self.demons.len());
                            // We move the demon to a thread
                            let answers_tx_clone = answers_tx.clone();
                            handles.insert(idx.clone(), tokio::spawn(async move {
                                #[cfg(feature = "full_log")]
                                log::debug!("[{}] calling handle function", demon.id());
                                let output = demon.handle(*input).await;
                                #[cfg(feature = "full_log")]
                                log::debug!("[{}] handle function called", demon.id());

                                // We first send the reply
                                if tx.send(Ok(Box::new(output))).is_err() {
                                    #[cfg(feature = "full_log")]
                                    log::error!("[{}] demon processed message could not be sent back", demon.id());
                                }

                                // Now the demon back
                                #[cfg(feature = "full_log")]
                                let demon_id = demon.id();
                                if answers_tx_clone.send((idx, demon)).is_err() {
                                    #[cfg(feature = "full_log")]
                                    log::error!("[{}] demon could not be sent back for reuse", demon_id);
                                }
                            }));
                        } else {
                            #[cfg(feature = "full_log")]
                            log::debug!("[{}] all demons are busy, puting message in inner queue. Total pending messages: {}", <D as Demon>::multiple_id(), requests.len() + 1);
                            requests.push_back((tx, *input));
                        }
                    } else {
                        if tx.send(Err(Error::WrongType)).is_err() {
                            #[cfg(feature = "full_log")]
                            log::error!("[{}] somehow, demon received wrong message type", <D as Demon>::multiple_id());   
                        }
                    }
                } else {
                    #[cfg(feature = "full_log")]
                    log::debug!("[{}] all incoming channels closed (impossible)", <D as Demon>::multiple_id());
                    break None;
                },
                res = self.instructions.recv() => match res {
                    Some(instruction) => match instruction {
                        MiniHellInstruction::Shutdown(vanquish_mailbox) => {
                            #[cfg(feature = "full_log")]
                            log::debug!("[{}] shutdown signal received", <D as Demon>::multiple_id());
                            break Some(vanquish_mailbox);
                        },
                        MiniHellInstruction::Message(result_mailbox, message) => {
                            #[cfg(feature = "full_log")]
                            log::debug!("[{}] received instruction, adding to the processing queue", <D as Demon>::multiple_id());
                            if mailbox.send((result_mailbox, message)).is_err() {
                                #[cfg(feature = "full_log")]
                                log::warn!("[{}] impossible error happened, could not send back message to itself!", <D as Demon>::multiple_id());   
                            }
                        }
                    },
                    None => {
                        #[cfg(feature = "full_log")]
                        log::info!("[{}] all channels to this demon are now closed", <D as Demon>::multiple_id());
                        break None;
                    }
                }
            }
        };

        // We call the vanquished function from this demon
        for (_, demon) in self.demons {
            #[cfg(feature = "full_log")]
            let demon_id = demon.id();
            #[cfg(feature = "full_log")]
            log::debug!("[{}] calling vanquish function", demon_id);
            demon.vanquished().await;
            #[cfg(feature = "full_log")]
            log::debug!("[{}] vanquish function called", demon_id);
        }

        if let Some(vanquish_mailbox) = vanquish_mailbox {
            if vanquish_mailbox.send(()).is_err() {
                #[cfg(feature = "full_log")]
                log::warn!("[{}] could not notify back hell about shutdown!", <D as Demon>::multiple_id());
            }
        }
    }
}