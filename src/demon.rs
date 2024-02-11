use std::future::Future;
pub use self::location::Location;
mod location;

/// Demon trait
///
/// Demons are actors in the apocalypse framework. Implement this trait in your actors to allow them to reply to messages.
pub trait Demon: Sized + std::marker::Send + 'static{
    type Input;
    type Output;

    /// Function that is called when a demon is spawned
    ///
    /// By default, the function does nothing.
    ///
    /// ```rust,no_run
    /// use apocalypse::{Demon, Location};
    ///
    /// struct EchoBot;
    ///
    /// impl Demon for EchoBot {
    ///     type Input = String;
    ///     type Output = String;
    ///     
    ///     // Callback for demon spawning
    ///     async fn spawned(&mut self, location: Location<Self>) {
    ///         log::debug!("Spawned echo bot with location {}", location);
    ///     }
    ///     
    ///     // Basic implementation of an echo handle function
    ///     async fn handle(&mut self, message: Self::Input) -> Self::Output {
    ///         message
    ///     }
    /// }
    /// ```
    fn spawned(&mut self, _location: Location<Self>) -> impl Future<Output = ()> + Send {
        async {}
    }

    /// Handler function for messages
    ///
    /// This is the main function, called for every message that the broker receives.
    ///
    /// ```rust,no_run
    /// use apocalypse::Demon;
    ///
    /// struct EchoBot;
    ///
    /// impl Demon for EchoBot {
    ///     type Input = String;
    ///     type Output = String;
    /// 
    ///     // Basic implementation of an echo handle function
    ///     async fn handle(&mut self, message: Self::Input) -> Self::Output {
    ///         message
    ///     }
    /// }
    /// ```
    fn handle(&mut self, message: Self::Input) -> impl Future<Output = Self::Output> + Send;

    /// Function that is called when a demon is removed
    ///
    /// By default, the function does nothing.
    ///
    /// ```rust,no_run
    /// use apocalypse::{Demon, Location};
    ///
    /// struct EchoBot;
    ///
    /// impl Demon for EchoBot {
    ///     type Input = String;
    ///     type Output = String;
    ///     
    ///     // Basic implementation of an echo handle function
    ///     async fn handle(&mut self, message: Self::Input) -> Self::Output {
    ///         message
    ///     }
    ///
    ///     // Callback function 
    ///     async fn vanquished(self) {
    ///         log::debug!("Killed echo bot");
    ///     }
    /// }
    /// ```
    fn vanquished(self) -> impl Future<Output = ()> + Send {
        async {}
    }

    /// This id will be printed in the debug logs of the demon's thread.
    ///
    /// It is useful when some lockup is happening and you have trouble to find it.
    #[cfg(feature = "full_log")]
    fn id(&self) -> String {
        "".to_string()
    }

    /// This id will be printed in the debug logs of the demon's thread, in case the [spawn_multiple](crate::Gate::spawn_multiple) function is used.
    ///
    /// It is useful when some lockup is happening and you have trouble to find it.
    #[cfg(feature = "full_log")]
    fn multiple_id() -> &'static str {
        ""
    }
}