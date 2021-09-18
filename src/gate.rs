use crate::{Error, Demon, Location, hell::{MiniHell, HellInstruction}};
use tokio::sync::{mpsc::UnboundedSender, oneshot::{self}};
use std::marker::PhantomData;
#[cfg(feature = "ws")]
use cataclysm_ws::{WebSocketReader};
#[cfg(feature = "ws")]
use tokio::net::tcp::OwnedReadHalf;
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
/// That is, dropping all gates finalize hell's execution. This structure cannot be created without the help of a [Hell](crate::Hell) instance.
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
    /// struct EchoDemon{}
    ///
    /// #[async_trait::async_trait]
    /// impl Demon for EchoDemon {
    ///     type Input = &'static str;
    ///     type Output = String;
    ///     async fn handle(&mut self, message: Self::Input) -> Self::Output {
    ///         format!("{}", message)
    ///     }
    /// }
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let hell = Hell::new();
    ///     let (gate, join_handle) = hell.fire().await.unwrap();
    ///     // we spawn the demon
    ///     let location = gate.spawn(EchoDemon{}).await.unwrap();
    ///     // In order to prevent lock ups, we send this future to another task
    ///     tokio::spawn(async move {
    ///         println!("{}", gate.send(&location, "Hallo, welt!").await.unwrap());
    ///     });
    ///     // We await the system
    ///     join_handle.await.unwrap();
    /// }
    /// ```
    pub async fn send<D, I, O>(&self, location: &Location<D>, message: I) -> Result<O, Error> 
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
    /// struct EchoDemon{}
    ///
    /// #[async_trait::async_trait]
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
    ///     let (gate, join_handle) = hell.fire().await.unwrap();
    ///     // we spawn the demon
    ///     let location = gate.spawn(EchoDemon{}).await.unwrap();
    ///     // In order to prevent lock ups, we send this future to another task
    ///     tokio::spawn(async move {
    ///         gate.send_and_ignore(&location, "Hallo, welt!").unwrap();
    ///     });
    ///     // We await the system
    ///     join_handle.await.unwrap();
    /// }
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
    /// ```rust,no_run
    /// use apocalypse::{Hell, Demon};
    ///
    /// struct Basic{}
    ///
    /// #[async_trait::async_trait]
    /// impl Demon for Basic {
    ///     type Input = ();
    ///     type Output = ();
    ///     async fn handle(&mut self, message: Self::Input) -> Self::Output {
    ///         println!("Hello, world!");
    ///     }
    /// }
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let hell = Hell::new();
    ///     let (gate, join_handle) = hell.fire().await.unwrap();
    ///     // we spawn the demon
    ///     let _location = gate.spawn(Basic{}).await;
    ///     // And do some other stuff
    ///     // ...
    ///     // We await the system
    ///     join_handle.await.unwrap();
    /// }
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
        let channel = MiniHell::spawn(demon, location.clone());

        // Second return channel, for knowing if the registration was successful
        let (tx, rx) = oneshot::channel();

        // We attempt the registration process
        self.hell_channel.send(HellInstruction::RegisterDemon {
            address,
            channel,
            tx
        }).map_err(|e| Error::TokioSend(format!("{}", e)))?;

        // If it returned true, then everything is ok
        rx.await.map_err(|s| Error::TokioSend(format!("{}", s)))?.map(move |_| location)
    }

    /// Spawns a demon with websockets processing in hell
    ///
    /// ```rust,no_run
    /// use apocalypse::{Hell, Demon};
    /// use cataclysm_ws::{WebSocketReader, Message};
    ///
    /// struct Basic{}
    ///
    /// #[async_trait::async_trait]
    /// impl Demon for Basic {
    ///     type Input = ();
    ///     type Output = ();
    ///     async fn handle(&mut self, message: Self::Input) -> Self::Output {
    ///         println!("Hello, world!");
    ///     }
    /// }
    /// 
    /// #[async_trait::async_trait]
    /// impl WebSocketReader for Basic {
    ///     async fn on_message(&mut self, message: Message) {
    ///         // ... do nothing
    ///     }
    /// }
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let hell = Hell::new();
    ///     let (gate, join_handle) = hell.fire().await.unwrap();
    ///     // In order to spawn, you should be able to obtain a
    ///     // OwnedHalfRead tcp stream from tokio, which is already 
    ///     // past the handshake protocol
    ///     // -> let _location = gate.spawn_ws(Basic{}, read_stream).await;
    /// }
    /// ```
    #[cfg(feature = "ws")]
    pub async fn spawn_ws<D: 'static + Demon<Input = I, Output = O> + WebSocketReader, I: 'static + Send, O: 'static + Send>(&self, demon: D, read_stream: OwnedReadHalf) -> Result<Location<D>, Error> {
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
        let channel = MiniWSHell::spawn(demon, location.clone(), read_stream);

        // Second return channel, for knowing if the registration was successful
        let (tx, rx) = oneshot::channel();

        // We attempt the registration process
        self.hell_channel.send(HellInstruction::RegisterDemon {
            address,
            channel,
            tx
        }).map_err(|e| Error::TokioSend(format!("{}", e)))?;

        // If it returned true, then everything is ok
        rx.await.map_err(|s| Error::TokioSend(format!("{}", s)))?.map(move |_| location)
    }

    /// Get rid of one demon
    ///
    /// With this method, you force one demon to be dropped. Notice that locations will not automatically reflect this change, and further messages sent to the dropped demon will return `Error::InvalidLocation`.
    ///
    /// ```rust,no_run
    /// use apocalypse::{Hell, Demon};
    ///
    /// struct EchoDemon{}
    ///
    /// #[async_trait::async_trait]
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
    ///     let (gate, join_handle) = hell.fire().await.unwrap();
    ///     // we spawn the demon
    ///     let location = gate.spawn(EchoDemon{}).await.unwrap();
    ///     // In order to prevent lock ups, we send this future to another task
    ///     gate.vanquish(&location).await.unwrap();
    ///     // We await the system
    ///     join_handle.await.unwrap();
    /// }
    /// ```
    pub async fn vanquish<D: 'static + Demon<Input = I, Output = O>, I: 'static + Send, O: 'static + Send>(&self, location: &Location<D>) -> Result<(), Error> {
        let (tx, rx) = oneshot::channel();
        self.hell_channel.send(HellInstruction::RemoveDemon{
            address: location.address,
            tx
        }).map_err(|e| Error::TokioSend(format!("{}", e)))?;
        rx.await.map_err(|e| Error::TokioSend(format!("{}", e)))?
    }

    /// Get rid of one demon, and ignore the result
    ///
    /// As with [send](crate::Gate::send) and [send_and_ignore](crate::Gate::send_and_ignore), this method is prefered because there is a lower chance of a lockup happening. For example, if you were to allow your own demon to vanquish itself, you should use this method.
    ///
    /// ```rust,no_run
    /// use apocalypse::{Hell, Demon};
    ///
    /// struct EchoDemon{}
    ///
    /// #[async_trait::async_trait]
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
    ///     let (gate, join_handle) = hell.fire().await.unwrap();
    ///     // we spawn the demon
    ///     let location = gate.spawn(EchoDemon{}).await.unwrap();
    ///     // In order to prevent lock ups, we send this future to another task
    ///     gate.vanquish_and_ignore(&location).unwrap();
    ///     // We await the system
    ///     join_handle.await.unwrap();
    /// }
    /// ```
    pub fn vanquish_and_ignore<D: 'static + Demon<Input = I, Output = O>, I: 'static + Send, O: 'static + Send>(&self, location: &Location<D>) -> Result<(), Error> {
        let (tx, rx) = oneshot::channel();
        self.hell_channel.send(HellInstruction::RemoveDemon{
            address: location.address,
            tx
        }).map_err(|e| Error::TokioSend(format!("{}", e)))?;
        tokio::spawn(async move{
            let _ = rx.await;
        });
        Ok(())
    }
}