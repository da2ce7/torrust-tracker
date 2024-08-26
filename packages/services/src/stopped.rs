//! Stopped Service - A service that is not starting, running, or halting.
//!

use crate::registration::ServiceRegistrationForm;
use crate::service::Service;
use crate::starting::Starting;

pub struct Stopped<S> {
    pub(super) stopped_service: S,
}

impl<S> Stopped<S>
where
    S: Service,
{
    /// Starts the Service
    ///
    /// # Errors
    ///
    /// This function will return an error if unable to start the service.
    pub fn start<'a>(self, form: ServiceRegistrationForm) -> Result<Starting<'a, S>, S::Error> {
        let starting_service = self.stopped_service.start()?;

        Ok(Starting {
            starting_service,
            form: Some(form),
        })
    }
}
