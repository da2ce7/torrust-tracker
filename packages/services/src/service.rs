//! Service Trait - A service that can be started and stopped.
//!

use std::error::Error;

use futures::future::BoxFuture;

use crate::registration::Registration;

pub trait Service: Sized {
    type Launcher;
    type Error: Error;

    /// Creates the new service from a launcher.
    ///
    /// # Errors
    ///
    /// This function will return an error if unable to create the new service.
    fn new(launcher: Self::Launcher) -> Result<Self, Self::Error>;

    /// Moves the service into the starting state.
    ///
    /// # Errors
    ///
    /// This function will return an error if the services is already started or stating
    #[allow(clippy::type_complexity)]
    fn start<'a>(self) -> Result<BoxFuture<'a, Result<(Self, Registration), Self::Error>>, Self::Error>;

    /// Moves the service into the stopping state
    ///
    /// # Errors
    ///
    /// This function will return an error if the services is not already started.
    fn stop<'a>(self) -> Result<BoxFuture<'a, Result<Self, Self::Error>>, Self::Error>;
}
