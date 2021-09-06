pub use self::location::Location;
pub(crate) use self::any_demon::AnyDemon;
mod location;
mod any_demon;

/// Demon trait
///
/// Demons are actors in the apocalyptic framework. Implement this trait in your actors to allow them to reply to messages.
#[async_trait::async_trait]
pub trait Demon: std::marker::Send {
    type Input;
    type Output;
    /// Handler function for messages
    async fn handle(&mut self, message: Self::Input) -> Self::Output;
}