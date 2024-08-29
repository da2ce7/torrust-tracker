//! Service Registration Module
//!
//! This module provides the necessary traits and structures for registering and deregistering services
//! It is a crucial part of the service lifecycle management, ensuring that services are properly
//! cataloged and checked accordingly.
//!
//! The key components of this module are:
//!
//! - [`ServiceCheck`]: A trait that defines a stream of health check results for a service.
//! - [`Registration`]: A trait that defines the registration process for a service, including health checks.
//! - [`ServiceDeregistration`]: A trait that defines the deregistration process for a service.
//! - [`ServiceRegistrationForm`]: A trait that defines the form used to register a service.
//! - [`ServiceDeregistrationForm`]: A trait that defines the form used to deregister a service.

use futures::future::BoxFuture;

/// A trait that defines a stream of health check results for a service.
/// Implementations of this trait should provide meaningful debug and display output,
/// and should be able to be sent across threads.
pub trait ServiceCheck:
    std::fmt::Debug + std::fmt::Display + futures::Stream<Item = Result<Self::Success, Self::Error>> + Send
{
    type Success;
    type Error;
}

/// A trait that defines the registration process for a service, including health checks.
/// Implementations of this trait should provide meaningful debug and display output.
pub trait Registration: std::fmt::Debug + std::fmt::Display + Send + 'static {
    type Success;
    type Error;

    /// Provides the health check for the service
    #[allow(clippy::type_complexity)]
    fn check(
        &mut self,
    ) -> Option<Box<dyn ServiceCheck<Success = Self::Success, Error = Self::Error, Item = Result<Self::Success, Self::Error>>>>;
}

/// A trait that defines the deregistration process for a service.
/// Implementations of this trait should provide meaningful debug and display output.
pub trait ServiceDeregistration: std::fmt::Debug {
    type Key;
}

/// A trait that defines the form used to register a service.
/// This trait is generic over the deregistration type `D`.
pub trait ServiceRegistrationForm<'b, CheckSuccess, CheckError> {
    type DeReg;

    /// The error if unable to register.
    type Error: std::error::Error;

    /// Registers the service using the provided registration.
    ///
    /// # Errors
    ///
    /// This function will return an error if the registration fails.
    #[allow(clippy::type_complexity)]
    fn register(
        self,
        registration: Box<dyn Registration<Success = CheckSuccess, Error = CheckError>>,
    ) -> Result<BoxFuture<'b, Result<Option<Self::DeReg>, Self::Error>>, Self::Error>;
}

/// A trait that defines the form used to deregister a service.
/// This trait is generic over the deregistration type `D`.
pub trait ServiceDeregistrationForm<'b, DeReg> {
    type DeReg: ServiceDeregistration;
    /// The error if unable to deregister.
    type Error: std::error::Error;

    /// Deregisters the service using the provided deregistration.
    ///
    /// # Errors
    ///
    /// This function will return an error if the deregistration fails.
    #[allow(clippy::type_complexity)]
    fn deregister(
        self,
        deregistration: DeReg,
    ) -> Result<BoxFuture<'b, Result<(), Box<dyn std::error::Error + Send>>>, Self::Error>;
}
