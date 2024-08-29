//! Stopped Service - A service that is not starting, running, or halting.
//!
//! This module contains the [`Stopped`] struct and related functionality for services that are not currently running.
//! The [`Stopped`] state represents the initial or terminal state of a service's lifecycle.
//!
//! The primary functionality provided by this module includes:
//!
//! - The [`Stopped`] struct: Represents a service that is not currently running.
//! - The [`start`][Stopped::start] method: Initiates the service, transitioning it from the [`Stopped`] state to the [`Starting`] state, which is a future representing the service in the process of being started.

use std::marker::PhantomData;

use super::starting::Starting;
use crate::service::Service;
use crate::ServiceRegistrationForm;

/// Represents a service that is not currently running.
pub struct Stopped<S> {
    pub(super) stopped_service: S,
}

impl<S> Stopped<S>
where
    S: Service,
{
    /// Starts the Service
    ///
    /// This method transitions the service from the [`Stopped`] state to the [`Starting`] state, which is a future representing the service in the process of being started.
    ///
    /// # Errors
    ///
    /// This function will return an error if unable to start the service.
    #[allow(clippy::type_complexity)]
    pub fn start<'a, 'b, Reg, RegError, Form, CheckSuccess, CheckError>(
        self,
        form: Form,
    ) -> Result<Starting<'a, 'b, S, Reg, RegError, Form, CheckSuccess, CheckError>, S::Error>
    where
        Form: ServiceRegistrationForm<'b, CheckSuccess, CheckError, Error = RegError>,
    {
        let starting_service = self.stopped_service.start()?;

        Ok(Starting {
            starting_service,
            form: Some(form),
            registration: None,
            running_service: None,
            _check_success: PhantomData,
            _check_error: PhantomData,
        })
    }
}
