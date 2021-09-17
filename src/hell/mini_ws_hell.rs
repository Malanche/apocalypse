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
        // Inner message passing
        let (mailbox, mut messages) = mpsc::unbounded_channel::<(Sender<Result<Box<dyn Any + Send>, Error>>, Box<dyn Any + Send>)>();
        // We prepare out websockets buffer
        let mut buf = BytesMut::with_capacity(8 * 1024);

        // We call both opening callbacks, starting by the websockets one
        self.demon.on_open().await;
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
                        Err(e) => panic!("{}", e)
                    };

                    if let Some(frame) = maybe_frame {
                        // We got a correct message, we clear the buffer
                        buf.clear();
                        // And call the handler
                        if let Some(message) = frame.into_message() {
                            self.demon.on_message(message).await;
                        }
                    } else {
                        // Closing the connection in a nice way
                        break;
                    }
                } else {
                    // Closed connection!
                    if buf.is_empty() {
                        break;
                    } else {
                        // connection reset by peer!
                        log::debug!("connection reset by peer");
                        break
                    }
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
                        log::debug!("Instructions will no longer arrive to hosting hell, closing demon for address {}", self.location.address);
                        break;
                    }
                }
            }
        }

        #[cfg(feature = "internal_log")]
        log::debug!("Leaving mini hell");

        // The same, in reverse order
        self.demon.vanquished().await;
        self.demon.on_close().await;
    }
}