use std::marker::PhantomData;

/// Demon's location (to be able to send messages).
///
/// The Demon's Location will be handed in by the Portal's once the spawn is complete. You can clone the address in order to share it across threads. This structure's only purpose is to help the portal know the types of the Input and Output that the Demon accespts.
pub struct Location<E: ?Sized> {
    /// Actual id of the demon
    pub(crate) address: usize,
    /// Phantom data just to make Rust happy
    pub(crate) phantom: PhantomData<E>
}

impl<E> Clone for Location<E> {
    fn clone(&self) -> Location<E> {
        Location {
            address: self.address,
            phantom: PhantomData
        }
    }
}