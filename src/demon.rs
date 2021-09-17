pub use self::location::Location;
mod location;

/// Demon trait
///
/// Demons are actors in the apocalyptic framework. Implement this trait in your actors to allow them to reply to messages.
#[async_trait::async_trait]
pub trait Demon: std::marker::Send {
    type Input;
    type Output;
    /// Handler function for messages
    async fn handle(&mut self, message: Self::Input) -> Self::Output;
    /// Callback, for when the demon is spawned
    async fn spawned(&mut self, _location: Location<Self>) {
        ()
    }
    /// Callback, for when the demon is dismissed
    async fn vanquished(&mut self) {
        ()
    }
}