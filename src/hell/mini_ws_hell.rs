use crate::{AnyWSDemon, Error};
use std::any::Any;
use tokio::{
    sync::{oneshot::Sender, mpsc::UnboundedReceiver},
    net::tcp::OwnedReadHalf,
    io::AsyncReadExt,
};
use cataclysm_ws::{Frame, Error as WSError};
use bytes::{BytesMut};

/// Structure that holds a single demon, and asynchronously deals with the messages that this demon receives.
pub(crate) struct MiniWSHell {
    /// Demon contained inside this minihell instance
    demon: Box<dyn AnyWSDemon>,
    /// Channel where to reply, and value to be fed to the demon
    queue: UnboundedReceiver<(Sender<Result<Box<dyn Any + Send>, Error>>, Box<dyn Any + Send>)>,
    /// Read stream where ws messages arrive
    read_stream: OwnedReadHalf,
}

impl MiniWSHell {
    pub fn spawn(demon: Box<dyn AnyWSDemon>, queue: UnboundedReceiver<(Sender<Result<Box<dyn Any + Send>, Error>>, Box<dyn Any + Send>)>, read_stream: OwnedReadHalf) {
        let mut mini_hell = MiniWSHell {
            demon,
            queue,
            read_stream
        };
        tokio::spawn(async move {
            mini_hell.demon.on_open().await;
            mini_hell.fire().await;
        });
    }

    async fn fire(mut self) {
        // We prepare out websockets buffer
        let mut buf = BytesMut::with_capacity(8 * 1024);
        loop {
            tokio::select! {
                pair = self.queue.recv() => if let Some((tx, message)) = pair {
                    let r = self.demon.handle_any(message).await;
                    if tx.send(r).is_err() {
                        #[cfg(feature = "debug")]
                        log::warn!("[Demon] I worked the answer, but I could not send it back...");   
                    }
                } else {
                    #[cfg(feature = "debug")]
                    log::info!("This demon is no longer needed, bye");
                    break;
                },
                bytes_read = self.read_stream.read_buf(&mut buf) => if 0 != bytes_read.unwrap() {
                    let maybe_frame = match Frame::parse(&buf) {
                        Ok(frame) => Some(frame),
                        Err(WSError::Parse(e)) => {
                            log::debug!("{}, clearing buffer", e);
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
                }
            }
        }

        self.demon.on_close().await;
    }
}