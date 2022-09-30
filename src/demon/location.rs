use std::marker::PhantomData;
use std::hash::{Hash, Hasher};

/// Demon's location (to be able to send messages).
///
/// The Demon's Location will be handed in by the Portal's once the spawn is complete. You can clone the address in order to share it across threads. This structure's only purpose is to help the portal know the types of the Input and Output that the Demon accespts.
pub struct Location<E: ?Sized> {
    /// Actual id of the demon
    pub(crate) address: usize,
    /// Phantom data just to make Rust happy
    pub(crate) phantom: PhantomData<E>
}

impl<A> AsRef<Location<A>> for Location<A> {
    fn as_ref(&self) -> &Location<A> {
        &self
    }
}

impl<E> Clone for Location<E> {
    fn clone(&self) -> Location<E> {
        Location {
            address: self.address,
            phantom: PhantomData
        }
    }
}

impl<E> Hash for Location<E> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.address.hash(state);
    }
}

impl<E> PartialEq for Location<E> {
    fn eq(&self, other: &Self) -> bool {
        self.address == other.address
    }
}

impl<E> Eq for Location<E>{}

impl<E> std::fmt::Display for Location<E> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        write!(formatter, "d-{}", self.address)
    }
}