use crate::{Error, Demon, AnyDemon, Location, hell::Action};
use tokio::sync::{mpsc::UnboundedSender, oneshot::{self, Sender}};
use std::future::{Future};
use std::marker::PhantomData;

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
    /// Message queue
    message_sink: UnboundedSender<Action>,
    /// Demon queue
    demon_sink: UnboundedSender<(Sender<usize>, Box<dyn AnyDemon>)>
}

impl Clone for Gate {
    fn clone(&self) -> Self {
        Gate {
            message_sink: self.message_sink.clone(),
            demon_sink: self.demon_sink.clone()
        }
    }
}

impl Gate {
    /// Creates a new gate. For internal use only.
    pub(crate) fn new(message_sink: UnboundedSender<Action>, demon_sink: UnboundedSender<(Sender<usize>, Box<dyn AnyDemon>)>) -> Gate {
        Gate {
            message_sink,
            demon_sink
        }
    }

    /// Sends a message to a demon
    ///
    /// In this actor implementaton, all messages do have to return some kind of reply. You can ignore this behaviour by setting your `Output` to () in most situations.
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
    ///         gate.send(&location, "Hallo, welt!").await;
    ///     });
    ///     // We await the system
    ///     join_handle.await.unwrap();
    /// }
    /// ```
    pub fn send<D, I, O>(&self, location: &Location<D>, message: I) -> impl Future<Output = Result<O, Error>> 
        where 
            D: Demon<Input = I, Output = O>,
            I: 'static + Send,
            O: 'static + Send {
        // async channel to get the response
        let (tx, rx) = oneshot::channel();
        let address = location.address;
        
        let res = self.message_sink.send(Action::Invoke{
            tx,
            address,
            input: Box::new(message)
        }).map_err(|e| Error::TokioSend(format!("{}", e)));
        
        async move {
            res?; // Immediate throw of error if it did not send properly
            #[cfg(feature = "debug")]
            log::info!("Adding new message to the queue, for address {}", address);
            // One shot channel to get the address back
            let any_output = rx.await.map_err(|s| Error::TokioSend(format!("{}", s)))??;
            if let Ok(output) = any_output.downcast::<O>() {
                Ok(*output)
            } else {
                Err(Error::WrongType)
            }
        }
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
    pub fn spawn<D: 'static + Demon<Input = I, Output = O>, I: 'static + Send, O: 'static + Send>(&self, demon: D) -> impl Future<Output = Result<Location<D>, Error>> {
        let (tx, rx) = oneshot::channel();

        // So we don't capture self, we execute the sending right here, and deal with the error in the
        // async block
        let res = self.demon_sink.send((tx, Box::new(demon))).map_err(|e| Error::TokioSend(format!("{}", e)));

        async move {
            res?; // Throw the error!
            #[cfg(feature = "debug")]
            log::info!("Adding new demon to hell");
            // One shot channel to get the address back
            let address = rx.await.map_err(|s| Error::TokioSend(format!("{}", s)))?;

            Ok(Location {
                address,
                phantom: PhantomData
            })
        }
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
    ///     gate.vanquish(&location).unwrap();
    ///     // We await the system
    ///     join_handle.await.unwrap();
    /// }
    /// ```
    pub fn vanquish<D: 'static + Demon<Input = I, Output = O>, I: 'static + Send, O: 'static + Send>(&self, location: &Location<D>) -> Result<(), Error> {
        Ok(self.message_sink.send(Action::Kill{
            address: location.address,
        }).map_err(|e| Error::TokioSend(format!("{}", e)))?)
    }
}