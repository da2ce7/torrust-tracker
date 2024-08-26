//! Starting Service - A service that is in the process of being started.
//!

use std::future::Future;

use futures::future::BoxFuture;
use futures::FutureExt;

use crate::registration::{Registration, ServiceRegistrationForm};
use crate::running::Running;
use crate::service::Service;

pub struct Starting<'a, S>
where
    S: Service,
{
    pub(super) starting_service: BoxFuture<'a, Result<(S, Registration), S::Error>>,
    pub(super) form: Option<ServiceRegistrationForm>,
}

impl<'a, S> Future for Starting<'a, S>
where
    S: Service,
{
    type Output = Result<Running<S>, S::Error>;

    fn poll(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        match self.starting_service.poll_unpin(cx) {
            std::task::Poll::Ready(Ok((running_service, registration))) => {
                let registration_id = registration.local_addr();

                let form = self.form.take().expect("it should have a form");

                let () = match form.send(registration) {
                    Ok(()) => (),
                    Err(e) => tracing::warn!(%e, "failed to send registration"),
                };

                std::task::Poll::Ready(Ok(Running {
                    running_service,
                    registration_id,
                }))
            }
            std::task::Poll::Ready(Err(e)) => std::task::Poll::Ready(Err(e)),
            std::task::Poll::Pending => std::task::Poll::Pending,
        }
    }
}
