//! Registar. Registers Services for Health Check.

use std::sync::Arc;

use thiserror::Error;

use crate::service_registry::ServiceRegistry;
use crate::{deregistration, registration};

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to get new registry reference")]
    FailedToGetRegister,
}

/// The [`Registar`] manages the [`ServiceRegistry`].
#[derive(Debug)]
pub struct Registar<Registry> {
    registry: Registry,
}

impl<Registry> Registar<Registry>
where
    Registry: ServiceRegistry,
{
    #[must_use]
    pub fn new() -> (Self, Arc<Registry>) {
        let registry = Registry::new();
        (
            Self {
                registry: registry.as_ref().clone(),
            },
            registry,
        )
    }

    /// Registers a Service
    ///
    /// # Errors
    ///
    /// This function will return an error if it fails to get a new registry reference.
    pub fn give_register_form(&self) -> Result<registration::Form<Registry>, Error> {
        let Some(registry) = self.registry.me().upgrade() else {
            return Err(Error::FailedToGetRegister);
        };
        Ok(registration::Form::new(registry))
    }

    /// Deregisters a Service
    ///
    /// # Errors
    ///
    /// This function will return an error if it fails to get a new registry reference.
    pub fn give_deregister_form(&self) -> Result<deregistration::Form<Registry>, Error> {
        let Some(registry) = self.registry.me().upgrade() else {
            return Err(Error::FailedToGetRegister);
        };
        Ok(deregistration::Form::new(registry))
    }
}
