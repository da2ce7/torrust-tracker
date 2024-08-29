//! Running Service - A service that is currently running.
//!
//! This module contains the [`Running`] struct and related functionality for services that are currently running.
//! The [`Running`] state is responsible for managing a service that is actively performing its tasks.
//!
//! The primary functionality provided by this module includes:
//!
//! - The [`Running`] struct: Represents a service that is currently running.
//! - The [`stop`][Running::stop] method: Transitions the service from the [`Running`] state to the [`Halting`] state.
//! - The [`deregister_and_stop`][Running::deregister_and_stop] method: Deregisters the service and then transitions it to the [`Halting`] state.
//! - The [`Stream`] implementation: Provides a stream of errors from the running service.

use futures::future::BoxFuture;
use futures::{Stream, StreamExt};
use pin_project::pin_project;

use super::halting::Halting;
use crate::registration::{ServiceDeregistration, ServiceDeregistrationForm};
use crate::service::Service;

#[pin_project]
pub struct Running<S, D> {
    #[pin]
    pub(super) running_service: S,
    pub(super) deregistration: Option<D>,
}

impl<S, DeReg> Running<S, DeReg>
where
    S: Service,
{
    /// Stops the running service.
    ///
    /// This method transitions the service from the [`Running`] state to the [`Halting`] state.
    ///
    /// # Errors
    ///
    /// This function will return an error if unable to send the stop message.
    pub fn stop<'a>(
        self,
        deregistration: Option<BoxFuture<'_, Result<(), Box<dyn std::error::Error + Send>>>>,
    ) -> Result<Halting<'a, '_, S>, S::Error> {
        let halting_service = self.running_service.stop()?;

        Ok(Halting {
            deregistration,
            halting_service,
        })
    }
}

impl<S, DeReg> Running<S, DeReg>
where
    S: Service,
    DeReg: ServiceDeregistration,
{
    /// Deregister and Stop the running service.
    ///
    /// This method deregisters the service and then transitions it from the [`Running`] state to the [`Halting`] state.
    ///
    /// # Errors
    ///
    /// This function will return an error if unable to send the stop message.
    pub fn deregister_and_stop<'a, 'b, Form>(mut self, form: Form) -> Result<Halting<'a, 'b, S>, S::Error>
    where
        Form: ServiceDeregistrationForm<'b, DeReg>,
    {
        let Some(deregistration) = self.deregistration.take() else {
            return self.stop(None);
        };

        let deregistration = match form.deregister(deregistration) {
            Ok(deregistration) => Some(deregistration),
            Err(e) => {
                tracing::warn!(%e, "failed to deregister service");
                None
            }
        };

        self.stop(deregistration)
    }
}

/// Note: if this stream returns [`None`], it signals that the service has failed and can be aborted.
impl<S, DeReg> Stream for Running<S, DeReg>
where
    S: Service + Stream<Item = S::Error> + Unpin,
{
    type Item = S::Error;

    fn poll_next(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Option<Self::Item>> {
        self.running_service.poll_next_unpin(cx)
    }
}
