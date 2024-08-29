//! Halting Service - A service that is in the process of being stopped.
//!
//! This module contains the [`Halting`] future that represents a service in the process of being stopped.
//! The [`Halting`] state is responsible for transitioning a service from the [`Running`][super::Running] state to the [`Stopped`] state.
//!
//! The primary functionality provided by this module includes:
//!
//! - The [`Halting`] future: Represents a service that is in the process of being stopped. Upon completion, it returns the [`Stopped`] structure.

use std::future::Future;

use futures::future::BoxFuture;
use futures::FutureExt;

use super::stopped::Stopped;
use crate::service::Service;

/// A future that represents a service in the process of being stopped.
///
/// This future handles the transition from the [`Running`][super::Running] state to the [`Stopped`] state.
/// Upon successful completion, it returns the [`Stopped`] structure.
pub struct Halting<'a, 'b, S>
where
    S: Service,
{
    pub(super) deregistration: Option<BoxFuture<'b, Result<(), Box<dyn std::error::Error + Send>>>>,
    pub(super) halting_service: BoxFuture<'a, Result<S, S::Error>>,
}

impl<'a, 'b, S> Future for Halting<'a, 'b, S>
where
    S: Service,
{
    type Output = Result<Stopped<S>, S::Error>;

    fn poll(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        if let Some(ref mut deregistration) = &mut self.deregistration {
            let () = match deregistration.poll_unpin(cx) {
                std::task::Poll::Ready(Err(e)) => tracing::warn!(%e, "failed to deregister service"),
                std::task::Poll::Ready(_) => (),
                std::task::Poll::Pending => return std::task::Poll::Pending,
            };
        }

        match self.halting_service.poll_unpin(cx) {
            std::task::Poll::Ready(Ok(stopped_service)) => std::task::Poll::Ready(Ok(Stopped { stopped_service })),
            std::task::Poll::Ready(Err(e)) => std::task::Poll::Ready(Err(e)),
            std::task::Poll::Pending => std::task::Poll::Pending,
        }
    }
}
