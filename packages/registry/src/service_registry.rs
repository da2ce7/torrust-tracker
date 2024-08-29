use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Weak};

use torrust_tracker_services::ServiceCheck;

pub type Checker<CheckSuccess, CheckError> =
    Box<dyn ServiceCheck<Success = CheckSuccess, Error = CheckError, Item = Result<CheckSuccess, CheckError>>>;

/// The [`ServiceRegistry`] provides a database...
pub trait ServiceRegistry<CheckSuccess, CheckError>: std::fmt::Debug + Clone {
    type Key;

    /// Returns a new arc of itself, if receiving new requests, when this arc is unwrapped it signals that the registry is no longer active.
    fn new() -> Arc<Self>;

    /// Returns a new weak of itself, if receiving new requests, this may be upgraded to an arc, if accepting requests.
    fn me(&self) -> Weak<Self>;

    /// Registers a service check.
    ///
    /// # Errors
    ///
    /// This function will return an error if the registration fails.
    fn register(
        self: Arc<Self>,
        value: Checker<CheckSuccess, CheckError>,
    ) -> Result<Option<Self::Key>, Checker<CheckSuccess, CheckError>>;

    /// Deregisters a service check.
    ///
    /// # Errors
    ///
    /// This function will return an error if the deregistration fails.
    fn deregister(self: Arc<Self>, key: Self::Key) -> Result<(), Self::Key>;
}

pub trait GenNextKey {
    type Key;

    fn next_id(&self) -> Option<Self::Key>;
}

#[derive(Default)]
struct IdGenerator {
    counter: AtomicUsize,
}

impl GenNextKey for IdGenerator {
    type Key = usize;

    fn next_id(&self) -> Option<Self::Key> {
        Some(self.counter.fetch_add(1, Ordering::SeqCst))
    }
}
