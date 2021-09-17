pub use self::location::Location;
pub(crate) use self::any_demon::AnyDemon;
pub(crate) use self::demon_wrapper::DemonWrapper;
mod demon_wrapper;
mod location;
mod any_demon;

#[cfg(feature = "ws")]
pub(crate) use self::any_ws_demon::AnyWSDemon;
#[cfg(feature = "ws")]
mod any_ws_demon;

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