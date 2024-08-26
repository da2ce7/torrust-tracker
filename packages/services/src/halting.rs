//! Halting Service - A service that is in the process of being stopped.
//!

use std::future::Future;

use futures::future::BoxFuture;
use futures::FutureExt;

use crate::service::Service;
use crate::stopped::Stopped;

pub struct Halting<'a, S>
where
    S: Service,
{
    pub(super) halting_service: BoxFuture<'a, Result<S, S::Error>>,
}

impl<'a, S> Future for Halting<'a, S>
where
    S: Service,
{
    type Output = Result<Stopped<S>, S::Error>;

    fn poll(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        match self.halting_service.poll_unpin(cx) {
            std::task::Poll::Ready(Ok(stopped_service)) => std::task::Poll::Ready(Ok(Stopped { stopped_service })),
            std::task::Poll::Ready(Err(e)) => std::task::Poll::Ready(Err(e)),
            std::task::Poll::Pending => std::task::Poll::Pending,
        }
    }
}
