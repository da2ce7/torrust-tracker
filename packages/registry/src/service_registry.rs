use std::sync::{Arc, Weak};

use torrust_tracker_services::ServiceCheck;

pub type Checker<CheckSuccess, CheckError> =
    Box<dyn ServiceCheck<Success = CheckSuccess, Error = CheckError, Item = Result<CheckSuccess, CheckError>>>;

/// The [`ServiceRegistry`] provides a database...
pub trait ServiceRegistry: std::fmt::Debug + Clone {
    type Key;
    type CheckSuccess;
    type CheckError;

    /// Returns a new arc of itself, if receiving new requests, when this arc is unwrapped it signals that the registry is no longer active.
    fn new() -> Arc<Self>;

    /// Returns a new weak of itself, if receiving new requests, this may be upgraded to an arc, if accepting requests.
    fn me(&self) -> Weak<Self>;

    /// Registers a service check.
    ///
    /// # Errors
    ///
    /// This function will return an error if the registration fails.
    #[allow(clippy::type_complexity)]
    fn register(
        self: Arc<Self>,
        value: Checker<Self::CheckSuccess, Self::CheckError>,
    ) -> Result<Option<Self::Key>, Checker<Self::CheckSuccess, Self::CheckError>>;

    /// Deregisters a service check.
    ///
    /// # Errors
    ///
    /// This function will return an error if the deregistration fails.
    fn deregister(self: Arc<Self>, key: Self::Key) -> Result<(), Self::Key>;
}
