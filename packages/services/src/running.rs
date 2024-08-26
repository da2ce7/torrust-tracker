use std::net::SocketAddr;

use futures::{Stream, StreamExt};

use crate::halting::Halting;
use crate::registration::ServiceDeregistrationForm;
use crate::service::Service;

pub struct Running<S> {
    pub(super) running_service: S,
    pub(super) registration_id: SocketAddr,
}

impl<S> Running<S>
where
    S: Service,
{
    /// Stops the running service.
    ///
    /// # Errors
    ///
    /// This function will return an error if unable to send the stop message.
    pub fn stop<'a>(self, form: ServiceDeregistrationForm) -> Result<Halting<'a, S>, S::Error> {
        let () = match form.send(self.registration_id) {
            Ok(()) => (),
            Err(e) => tracing::warn!(%e, "failed to deregister service"),
        };

        let halting_service = self.running_service.stop()?;

        Ok(Halting { halting_service })
    }
}

impl<S> Stream for Running<S>
where
    S: Service + Stream<Item = S::Error> + Unpin,
{
    type Item = S::Error;

    fn poll_next(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Option<Self::Item>> {
        self.running_service.poll_next_unpin(cx)
    }
}
