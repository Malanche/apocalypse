use crate::{Error, Demon, Location, hell::{MiniHell, MultipleMiniHell, HellInstruction, HellStats, Channel}};
use tokio::sync::{mpsc::UnboundedSender, oneshot::{self}};
use std::sync::mpsc;
use std::marker::PhantomData;
#[cfg(feature = "ws")]
use cataclysm::ws::{WebSocketThread, WebSocketReader};
#[cfg(feature = "ws")]
use crate::hell::MiniWSHell;

/// ## Gate structure
///
/// The portal structure allows communication with the demons, as well as demon spawning.
///
/// Gates are the main component of this library. They hold a channel of communication with their hell instance. It is important to notice that hell will continue to exist as long as any of the two following things are true
///
/// * There are messages still in the queue to be processed
/// * There is at least one gate alive
///
/// That is, dropping all gates finalizes hell's execution. Due to the fact that a gate is required to send messages, and some Demons will have a gate among their fields, you have to remove all Demons in posession of a Gate to shutdown Hell gracefully. This structure cannot be created without the help of a [Hell](crate::Hell) instance.
pub struct Gate {
    /// Communication with main hell instance
    hell_channel: UnboundedSender<HellInstruction>
}

impl Clone for Gate {
    fn clone(&self) -> Self {
        Gate {
            hell_channel: self.hell_channel.clone()
        }
    }
}

impl Gate {
    /// Creates a new gate. For internal use only.
    pub(crate) fn new(hell_channel: UnboundedSender<HellInstruction>) -> Gate {
        Gate {
            hell_channel,
        }
    }

    /// Sends a message to a demon
    ///
    /// In this actor implementaton, all messages do have to return some kind of reply. Be aware that this decision can lead to lockups if used carelessly (as the mutable access that the handle function has to the demons blocks the message processing loop until each handle call ends). If you manage to create a message-cycle (that is, a chain of requests that has as element the same actor twice), then you will end up in a lockup situation. Try to use this function **only** when necessary, keep [send_and_ignore](crate::Gate::send_and_ignore) as your first option, unless you carefully thought about the message-chains in your software.
    ///
    /// ```rust
    /// use apocalypse::{Hell, Demon};
    ///
    /// struct EchoBot;
    ///
    /// impl Demon for EchoBot {
    ///     type Input = &'static str;
    ///     type Output = String;
    ///     async fn handle(&mut self, message: Self::Input) -> Self::Output {
    ///         message.to_string()
    ///     }
    /// }
    ///
    /// # #[tokio::main]
    /// # async fn main() {
    /// let (gate, jh) = Hell::new().ignite().await.unwrap();
    /// let location = gate.spawn(EchoBot).await.unwrap();
    /// // Use the send function to send a message
    /// let message = gate.send(&location, "Hallo, welt!").await.unwrap();
    /// # }
    /// ```
    pub async fn send<A: AsRef<Location<D>>, D, I, O>(&self, location: A, message: I) -> Result<O, Error> 
        where 
            D: Demon<Input = I, Output = O>,
            I: 'static + Send,
            O: 'static + Send {
        // async channel to get the response
        let (tx, rx) = oneshot::channel();
        let address = location.as_ref().address;
        
        self.hell_channel.send(HellInstruction::Message {
            tx,
            address,
            input: Box::new(message)
        }).map_err(|e| Error::TokioSend(format!("hell channel error, {}", e)))?;

        let any_output = rx.await.map_err(|s| Error::TokioSend(format!("{}", s)))??;

        if let Ok(output) = any_output.downcast::<O>() {
            Ok(*output)
        } else {
            Err(Error::WrongType)
        }
    }

    /// Sends a message to a demon, and ignore the result.
    ///
    /// This is your go-to function when you don't have to wait for the actor to give you a response back.
    ///
    /// ```rust
    /// use apocalypse::{Hell, Demon};
    ///
    /// struct PrintBot;
    ///
    /// impl Demon for PrintBot {
    ///     type Input = &'static str;
    ///     type Output = ();
    ///     async fn handle(&mut self, message: Self::Input) {
    ///         println!("{}", message);
    ///     }
    /// }
    ///
    /// # #[tokio::main]
    /// # async fn main() {
    /// let (gate, jh) = Hell::new().ignite().await.unwrap();
    /// let location = gate.spawn(PrintBot).await.unwrap();
    /// // Use the send and ignore function to send a message without waiting for it
    /// gate.send_and_ignore(&location, "Hallo, welt!").unwrap();
    /// # }
    /// ```
    pub fn send_and_ignore<D, I, O>(&self, location: &Location<D>, message: I) -> Result<(), Error> 
        where 
            D: Demon<Input = I, Output = O>,
            I: 'static + Send,
            O: 'static + Send {
        // async channel to get the response
        let (tx, rx) = oneshot::channel();
        let address = location.address;
        
        self.hell_channel.send(HellInstruction::Message {
            tx,
            address,
            input: Box::new(message)
        }).map_err(|e| Error::TokioSend(format!("hell channel error, {}", e)))?;

        tokio::spawn(async move {
            let _ = rx.await;
        });
        Ok(())
    }

    /// Spawns a demon in hell
    ///
    /// ```rust
    /// use apocalypse::{Hell, Demon};
    ///
    /// struct Basic;
    ///
    /// impl Demon for Basic {
    ///     type Input = String;
    ///     type Output = ();
    ///     async fn handle(&mut self, message: Self::Input) -> Self::Output {
    ///         println!("Hello, world!");
    ///     }
    /// }
    ///
    /// # #[tokio::main]
    /// # async fn main() {
    /// let (gate, join_handle) = Hell::new().ignite().await.unwrap();
    /// // we spawn the demon
    /// let _location = gate.spawn(Basic).await.unwrap();
    /// // Do something
    /// # }
    /// ```
    pub async fn spawn<D: 'static + Demon<Input = I, Output = O>, I: 'static + Send, O: 'static + Send>(&self, demon: D) -> Result<Location<D>, Error> {
        // First return channel, to get a valid address
        let (tx, rx) = oneshot::channel();

        // We request an address
        self.hell_channel.send(HellInstruction::CreateAddress {
            tx
        }).map_err(|e| Error::TokioSend(format!("{}", e)))?;
        let address = rx.await.map_err(|s| Error::TokioSend(format!("{}", s)))?;

        let location = Location {
            address,
            phantom: PhantomData
        };

        // We spawn the demon in a mini hell instance
        let demon_channels = MiniHell::spawn(demon, location.clone());

        // Second return channel, for knowing if the registration was successful
        let (tx, rx) = oneshot::channel();

        // We attempt the registration process
        self.hell_channel.send(HellInstruction::RegisterDemon {
            address,
            demon_channels,
            tx
        }).map_err(|e| Error::TokioSend(format!("{}", e)))?;

        // If it returned true, then everything is ok
        rx.await.map_err(|s| Error::TokioSend(format!("{}", s)))?.map(move |_| location)
    }

    /// Spawns multiple demons in Hell, that reply to the same [Location](Location)
    ///
    /// This might be useful if you have one task that consumes some time to be processed, and you can also parallelize. The load balancing method is just using whichever Demon is free at the moment, in a sequential order (that is, sequential but skipping if one is busy).
    ///
    /// ```rust
    /// use apocalypse::{Hell, Demon};
    ///
    /// struct Basic;
    ///
    /// impl Demon for Basic {
    ///     type Input = String;
    ///     type Output = ();
    ///     async fn handle(&mut self, message: Self::Input) -> Self::Output {
    ///         println!("Hello, world!");
    ///     }
    /// }
    ///
    /// # #[tokio::main]
    /// # async fn main() {
    /// let (gate, join_handle) = Hell::new().ignite().await.unwrap();
    /// // Demon factory, needs to implement FnMut
    /// let basic_factory = || {
    ///     Basic
    /// };
    /// // We spawn three instances of the demon
    /// let _location = gate.spawn_multiple(basic_factory, 3).await.unwrap();
    /// // Do something
    /// # }
    /// ```
    pub async fn spawn_multiple<D: 'static + Demon<Input = I, Output = O>, I: 'static + Send, O: 'static + Send, F: FnMut() -> D>(&self, demon_factory: F, replicas: usize) -> Result<Location<D>, Error> {
        // First return channel, to get a valid address
        let (tx, rx) = oneshot::channel();

        // We request an address
        self.hell_channel.send(HellInstruction::CreateAddress {
            tx
        }).map_err(|e| Error::TokioSend(format!("{}", e)))?;
        let address = rx.await.map_err(|s| Error::TokioSend(format!("{}", s)))?;

        let location = Location {
            address,
            phantom: PhantomData
        };

        // We spawn the demon in a mini hell instance
        let demon_channels = MultipleMiniHell::spawn(demon_factory, replicas, location.clone())?;

        // Second return channel, for knowing if the registration was successful
        let (tx, rx) = oneshot::channel();

        // We attempt the registration process
        self.hell_channel.send(HellInstruction::RegisterDemon {
            address,
            demon_channels,
            tx
        }).map_err(|e| Error::TokioSend(format!("{}", e)))?;

        // If it returned true, then everything is ok
        rx.await.map_err(|s| Error::TokioSend(format!("{}", s)))?.map(move |_| location)
    }

    /// Spawns a demon with websockets processing in hell
    ///
    /// Demons spawned with this method need to implement the WebSocketThread trait. Demons will process both messages incoming from apocalypse, as well as from the websockets connection. It is important to note that the websockets handshake is not at all performed by this library.
    ///
    /// ```rust,no_run
    /// use apocalypse::{Hell, Demon};
    /// use cataclysm::ws::{WebSocketThread, Message};
    ///
    /// struct PrintBot;
    ///
    /// impl Demon for PrintBot {
    ///     type Input = ();
    ///     type Output = ();
    ///     async fn handle(&mut self, message: Self::Input) -> Self::Output {
    ///         println!("Hello, world!");
    ///     }
    /// }
    /// 
    /// impl WebSocketThread for PrintBot {
    ///     type Output = ();
    ///     async fn on_message(&mut self, message: Message) {
    ///         // ... do something with the message
    ///     }
    ///
    ///     async fn on_close(&mut self, _clean: bool) -> Self::Output {}
    /// }
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let hell = Hell::new();
    ///     let (gate, join_handle) = hell.ignite().await.unwrap();
    ///     // In order to spawn, you should be able to obtain a
    ///     // OwnedHalfRead tcp stream from tokio or similar
    ///     // -> let _location = gate.spawn_ws(Basic{}, read_stream).await;
    /// }
    /// ```
    #[cfg(feature = "ws")]
    pub async fn spawn_ws<D: 'static + Demon<Input = I, Output = O> + WebSocketThread, I: 'static + Send, O: 'static + Send>(&self, demon: D, wsr: WebSocketReader) -> Result<Location<D>, Error> {
        // First return channel, to get a valid address
        let (tx, rx) = oneshot::channel();

        // We request an address
        self.hell_channel.send(HellInstruction::CreateAddress {
            tx
        }).map_err(|e| Error::TokioSend(format!("{}", e)))?;
        let address = rx.await.map_err(|s| Error::TokioSend(format!("{}", s)))?;

        let location = Location {
            address,
            phantom: PhantomData
        };

        // We spawn the demon in a mini hell instance
        let demon_channels = MiniWSHell::spawn(demon, location.clone(), wsr);

        // Second return channel, for knowing if the registration was successful
        let (tx, rx) = oneshot::channel();

        // We attempt the registration process
        self.hell_channel.send(HellInstruction::RegisterDemon {
            address,
            demon_channels,
            tx
        }).map_err(|e| Error::TokioSend(format!("{}", e)))?;

        // If it returned true, then everything is ok
        rx.await.map_err(|s| Error::TokioSend(format!("{}", s)))?.map(move |_| location)
    }

    /// Get rid of one demon gracefully
    ///
    /// With this method, you request one demon to be dropped. Notice that locations will not automatically reflect this change, and further messages sent to the dropped demon will return `Error::InvalidLocation`. This method with block until the demon confirms is no longer executing anything. There is no guarantee that all pending messages will be processed before termination.
    /// 
    /// If the hell instance has a default `timeout` for vanquishing demons, this function will return the latest at `timeout`. If you want to override this behaviour for a single call, see [vanquish_with_timeout](Gate::vanquish_with_timeout).
    ///
    /// ```rust
    /// use apocalypse::{Hell, Demon};
    ///
    /// struct EchoDemon{}
    ///
    /// impl Demon for EchoDemon {
    ///     type Input = &'static str;
    ///     type Output = ();
    ///     async fn handle(&mut self, message: Self::Input) -> Self::Output {
    ///         println!("{}", message);
    ///     }
    /// }
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let hell = Hell::new();
    ///     let join_handle = {
    ///         // We ignite our hell instance in another span to guarantee our gate is dropped after use
    ///         let (gate, join_handle) = hell.ignite().await.unwrap();
    ///         // We spawn the echo demon
    ///         let location = gate.spawn(EchoDemon{}).await.unwrap();
    ///         // We wait until the demon fades away
    ///         gate.vanquish(&location).await.unwrap();
    ///         join_handle
    ///     };
    ///     // We await the system
    ///     join_handle.await.unwrap();
    /// }
    /// ```
    pub async fn vanquish<D: 'static + Demon<Input = I, Output = O>, I: 'static + Send, O: 'static + Send>(&self, location: &Location<D>) -> Result<(), Error> {
        let (tx, rx) = oneshot::channel();
        self.hell_channel.send(HellInstruction::RemoveDemon{
            address: location.address,
            tx: Channel::Async(tx),
            ignore: false,
            force: None
        }).map_err(|e| Error::TokioSend(format!("{}", e)))?;
        rx.await.map_err(|e| Error::TokioSend(format!("{}", e)))?
    }

    /// Get rid of one demon gracefully
    ///
    /// With this method, you request one demon to be dropped. Notice that locations will not automatically reflect this change, and further messages sent to the dropped demon will return `Error::InvalidLocation`. This method with block until the demon confirms is no longer executing anything. There is no guarantee that all pending messages will be processed before termination.
    /// 
    /// If the hell instance has a default `timeout` for vanquishing demons, this function will return the latest at `timeout`. If you want to override this behaviour for a single call, see [vanquish_with_timeout](Gate::vanquish_with_timeout).
    ///
    /// ```rust
    /// use apocalypse::{Hell, Demon};
    /// use std::time::Duration;
    ///
    /// struct EchoDemon{}
    ///
    /// impl Demon for EchoDemon {
    ///     type Input = &'static str;
    ///     type Output = ();
    ///     async fn handle(&mut self, message: Self::Input) -> Self::Output {
    ///         // The demon is slow, takes 2 seconds to reply
    ///         tokio::time::sleep(Duration::from_secs(2)).await;
    ///         println!("{}", message);
    ///     }
    /// }
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let hell = Hell::new();
    ///     let join_handle = {
    ///         // We ignite our hell instance in another span to guarantee our gate is dropped after use
    ///         let (gate, join_handle) = hell.ignite().await.unwrap();
    ///         // We spawn the echo demon
    ///         let location = gate.spawn(EchoDemon{}).await.unwrap();
    ///         // We wait maximum 1 second for the demon to finish whatever it is doing
    ///         gate.vanquish_with_timeout(&location, Some(Duration::from_secs(1))).await.unwrap();
    ///         join_handle
    ///     };
    ///     // We await the system
    ///     join_handle.await.unwrap();
    /// }
    /// ```
    pub async fn vanquish_with_timeout<D: 'static + Demon<Input = I, Output = O>, I: 'static + Send, O: 'static + Send>(&self, location: &Location<D>, timeout: Option<std::time::Duration>) -> Result<(), Error> {
        let (tx, rx) = oneshot::channel();
        self.hell_channel.send(HellInstruction::RemoveDemon{
            address: location.address,
            tx: Channel::Async(tx),
            ignore: false,
            force: Some(timeout)
        }).map_err(|e| Error::TokioSend(format!("{}", e)))?;
        rx.await.map_err(|e| Error::TokioSend(format!("{}", e)))?
    }

    /// Get rid of one demon gracefully, and ignore the result
    ///
    /// As with [send](crate::Gate::send) and [send_and_ignore](crate::Gate::send_and_ignore), this method is prefered because there is a lower chance of a lockup happening. For example, if you were to allow your own demon to vanquish itself, you should use this method. The method fails if the vanquish request does not reach the demon.
    ///
    /// If the hell instance has a default `timeout` for vanquishing demons, this function will return the latest at `timeout`. If you want to override this behaviour for a single call, see [vanquish_and_ignore_with_timeout](Gate::vanquish_and_ignore_with_timeout).
    ///
    /// ```rust
    /// use apocalypse::{Hell, Demon};
    ///
    /// struct EchoDemon{}
    ///
    /// impl Demon for EchoDemon {
    ///     type Input = &'static str;
    ///     type Output = ();
    ///     async fn handle(&mut self, message: Self::Input) -> Self::Output {
    ///         println!("{}", message);
    ///     }
    /// }
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let hell = Hell::new();
    ///     let join_handle = {
    ///         // We ignite our hell instance in another span to guarantee our gate is dropped after use
    ///         let (gate, join_handle) = hell.ignite().await.unwrap();
    ///         // We spawn the echo demon
    ///         let location = gate.spawn(EchoDemon{}).await.unwrap();
    ///         // This function fails if the vanquish request does not reach the demon, but if
    ///         // if it does, it does not block.
    ///         gate.vanquish_and_ignore(&location).unwrap();
    ///         join_handle
    ///     };
    ///     // We await the system
    ///     join_handle.await.unwrap();
    /// }
    /// ```
    pub fn vanquish_and_ignore<D: 'static + Demon<Input = I, Output = O>, I: 'static + Send, O: 'static + Send>(&self, location: &Location<D>) -> Result<(), Error> {
        let (tx, rx) = mpsc::channel();
        self.hell_channel.send(HellInstruction::RemoveDemon{
            address: location.address,
            tx: Channel::Sync(tx),
            ignore: true,
            force: None
        }).map_err(|e| Error::TokioSend(format!("{}", e)))?;
        rx.recv().map_err(Error::RecvError)?
    }

    /// Get rid of one demon, and ignore the result.
    ///
    /// As with [send](crate::Gate::send) and [send_and_ignore](crate::Gate::send_and_ignore), this method is prefered because there is a lower chance of a lockup happening. For example, if you were to allow your own demon to vanquish itself, you should use this method.
    ///
    /// ```rust
    /// use apocalypse::{Hell, Demon};
    /// use std::time::Duration;
    ///
    /// struct EchoDemon{}
    ///
    /// impl Demon for EchoDemon {
    ///     type Input = &'static str;
    ///     type Output = ();
    ///     async fn handle(&mut self, message: Self::Input) -> Self::Output {
    ///         // The demon is slow, takes 2 seconds to reply
    ///         tokio::time::sleep(Duration::from_secs(2)).await;
    ///         println!("{}", message);
    ///     }
    /// }
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let hell = Hell::new();
    ///     let join_handle = {
    ///         // We ignite our hell instance in another span to guarantee our gate is dropped after use
    ///         let (gate, join_handle) = hell.ignite().await.unwrap();
    ///         // We spawn the echo demon
    ///         let location = gate.spawn(EchoDemon{}).await.unwrap();
    ///         // We wait maximum 1 second for the demon to finish whatever it is doing
    ///         gate.vanquish_and_ignore_with_timeout(&location, Some(Duration::from_secs(1))).unwrap();
    ///         join_handle
    ///     };
    ///     // We await the system
    ///     join_handle.await.unwrap();
    /// }
    /// ```
    pub fn vanquish_and_ignore_with_timeout<D: 'static + Demon<Input = I, Output = O>, I: 'static + Send, O: 'static + Send>(&self, location: &Location<D>, timeout: Option<std::time::Duration>) -> Result<(), Error> {
        let (tx, rx) = mpsc::channel();
        self.hell_channel.send(HellInstruction::RemoveDemon{
            address: location.address,
            tx: Channel::Sync(tx),
            ignore: true,
            force: Some(timeout)
        }).map_err(|e| Error::TokioSend(format!("{}", e)))?;
        rx.recv().map_err(Error::RecvError)?
    }

    /// Stops the broker
    ///
    /// If wait is true, the broker will wait for every single demon to finish the vanquished method
    pub async fn extinguish(self, wait: bool) -> Result<(), Error>{
        let (tx, rx) = oneshot::channel();
        self.hell_channel.send(HellInstruction::Extinguish{tx, wait}).map_err(|e| Error::TokioSend(format!("{}", e)))?;
        rx.await.map_err(|e| Error::TokioSend(format!("{}", e)))?;
        Ok(())
    }

    /// Requests hell statistics
    ///
    /// This method returns a structure containing operation stats.
    pub async fn stats(&self) -> Result<HellStats, Error> {
        let (tx, rx) = oneshot::channel();
        self.hell_channel.send(HellInstruction::Stats{tx}).map_err(|e| Error::TokioSend(format!("{}", e)))?;
        rx.await.map_err(|e| Error::TokioSend(format!("{}", e)))
    }
}