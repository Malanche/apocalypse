use crate::{Error, Demon, Location, hell::MiniHellInstruction};
use std::any::Any;

use tokio::{
    sync::{oneshot::Sender, mpsc::{self, UnboundedSender, UnboundedReceiver}},
    net::tcp::OwnedReadHalf,
    io::AsyncReadExt,
};
use cataclysm_ws::{Frame, WebSocketReader, Error as WSError};
use bytes::{BytesMut};

/// Structure that holds a single demon, and asynchronously deals with the messages that this demon receives.
pub(crate) struct MiniWSHell<D> {
    /// Demon contained inside this minihell instance
    demon: D,
    /// Address of this demon
    location: Location<D>,
    /// Channel where instructions are sent to the minihell
    instructions: UnboundedReceiver<MiniHellInstruction>,
    /// Read stream where ws messages arrive
    read_stream: OwnedReadHalf
}

impl<I: 'static + Send, O: 'static + Send, D: 'static + Demon<Input = I, Output = O> + WebSocketReader> MiniWSHell<D> {
    pub fn spawn(demon: D, location: Location<D>, read_stream: OwnedReadHalf) -> UnboundedSender<MiniHellInstruction> {
        let (mailbox, instructions) = mpsc::unbounded_channel();
        let mini_hell = MiniWSHell {
            demon,
            location,
            instructions,
            read_stream
        };
        tokio::spawn(async move {
            mini_hell.fire().await;
        });
        mailbox
    }

    async fn fire(mut self) {
        #[cfg(feature = "internal_log")]
        log::debug!("[{}] demon thread starting", self.demon.id());
        // Inner message passing
        let (mailbox, mut messages) = mpsc::unbounded_channel::<(Sender<Result<Box<dyn Any + Send>, Error>>, Box<dyn Any + Send>)>();
        // We prepare out websockets buffer
        let mut buf = BytesMut::with_capacity(8 * 1024);

        // We call both opening callbacks, starting by the websockets one
        self.demon.on_open().await;
        let other_loc = self.location.clone();
        self.demon.spawned(other_loc).await;

        loop {
            #[cfg(feature = "internal_log")]
            log::debug!("[{}] start of await loop", self.demon.id());
            tokio::select! {
                res = messages.recv() => if let Some((tx, input)) = res {
                    if let Ok(input) = input.downcast::<I>() {
                        #[cfg(feature = "internal_log")]
                        log::warn!("[{}] demon handle processing message...", self.demon.id());
                        let output = self.demon.handle(*input).await;
                        if tx.send(Ok(Box::new(output))).is_err() {
                            #[cfg(feature = "internal_log")]
                            log::warn!("[{}] demon handle result could not be delivered back", self.demon.id());   
                        }
                    } else {
                        if tx.send(Err(Error::WrongType)).is_err() {
                            #[cfg(feature = "internal_log")]
                            log::warn!("[{}] demon handle received a wrong type and the message could not be delivered back", self.demon.id());   
                        }
                    }
                } else {
                    #[cfg(feature = "internal_log")]
                    log::info!("[{}] all channels to this demon are now closed (impossible)", self.demon.id());
                    break;
                },
                bytes_read = self.read_stream.read_buf(&mut buf) => if 0 != bytes_read.unwrap() {
                    let maybe_frame = match Frame::parse(&buf) {
                        Ok(frame) => Some(frame),
                        Err(WSError::Parse(_e)) => {
                            #[cfg(feature = "internal_log")]
                            log::debug!("{}, clearing buffer", _e);
                            buf.clear();
                            None
                        },
                        Err(WSError::Incomplete) => None,
                        Err(_e) => {
                            #[cfg(feature = "internal_log")]
                            log::debug!("[{}] error occured while parsing websockets frame, {}", self.demon.id(), _e);
                            None
                        }
                    };

                    if let Some(frame) = maybe_frame {
                        #[cfg(feature = "internal_log")]
                        log::debug!("[{}] received complete websockets message, processing", self.demon.id());
                        // We got a correct message, we clear the buffer
                        buf.clear();
                        // And call the handler
                        if let Some(message) = frame.into_message() {
                            self.demon.on_message(message).await;
                        }
                    } else {
                        // More data needs to arrive to parse the message  properly
                        continue;
                    }
                } else {
                    // Closed connection!
                    if buf.is_empty() {
                        // Half-clean exit
                        #[cfg(feature = "internal_log")]
                        log::info!("[{}] ws connection dropped", self.demon.id());
                        break;
                    } else {
                        // connection reset by peer! horrible connection
                        log::debug!("[{}] connection reset by peer, with some data in the buffer", self.demon.id());
                        break
                    }
                },
                res = self.instructions.recv() => match res {
                    Some(instruction) => match instruction {
                        MiniHellInstruction::Shutdown => {
                            #[cfg(feature = "internal_log")]
                            log::info!("[{}] shutdown signal received", self.demon.id());
                            break
                        },
                        MiniHellInstruction::Message(result_mailbox, message) => {
                            #[cfg(feature = "internal_log")]
                            log::debug!("[{}] received message, adding to the processing queue", self.demon.id());
                            if mailbox.send((result_mailbox, message)).is_err() {
                                #[cfg(feature = "internal_log")]
                                log::warn!("[{}] impossible error happened, could not send back message to itself!", self.demon.id());   
                            }
                        }
                    },
                    None => {
                        #[cfg(feature = "internal_log")]
                        log::info!("[{}] all channels to this demon are now closed", self.demon.id());
                        break;
                    }
                }
            }
            #[cfg(feature = "internal_log")]
            log::debug!("[{}] end of await loop", self.demon.id());
        }

        // The same, in reverse order
        self.demon.vanquished().await;
        self.demon.on_close().await;

        #[cfg(feature = "internal_log")]
        log::debug!("[{}] demon thread finished", self.demon.id());
    }
}