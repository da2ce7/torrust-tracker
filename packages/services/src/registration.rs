//! Service Registration. Registers Services for Health Check.
//!

use std::net::SocketAddr;
use std::pin::Pin;

use derive_more::derive::{Deref, Display};
use derive_more::Constructor;
use futures::channel::oneshot;
use futures::{Stream, StreamExt};

/// A [`ServiceHeathCheckResult`] is returned by a completed health check.
///
#[derive(Debug, Constructor, Deref)]
pub struct ServiceHeathCheckResult(Result<String, String>);

/// The trait [`ServiceHealthCheck`] has a health check job with it's metadata
///
/// The `job` awaits a [`ServiceHeathCheckResult`].
///
pub trait ServiceHealthCheck: std::fmt::Debug + std::fmt::Display + Stream<Item = ServiceHeathCheckResult> + Send {
    fn local_addr(&self) -> SocketAddr;
    fn info(&self) -> String;
}

/// A [`Registration`] is catalogued.
///
/// Each registration includes a check that fulfils the [`ServiceHealthCheck`] specification.
///
#[derive(Debug, Display)]
pub struct Registration {
    check: Pin<Box<dyn ServiceHealthCheck>>,
}

impl Registration {
    pub fn new<C>(check: C) -> Self
    where
        C: ServiceHealthCheck + 'static,
    {
        Self { check: Box::pin(check) }
    }

    #[must_use]
    pub fn local_addr(&self) -> SocketAddr {
        self.check.local_addr()
    }

    #[must_use]
    pub async fn spawn_check(&mut self) -> Option<ServiceHeathCheckResult> {
        self.check.next().await
    }
}

/// A [`ServiceRegistrationForm`] will return a completed [`Registration`].
///
#[derive(Constructor)]
pub struct ServiceRegistrationForm(oneshot::Sender<Registration>);

impl ServiceRegistrationForm {
    pub(crate) fn send(self, registration: Registration) -> Result<(), Registration> {
        self.0.send(registration)
    }
}

/// A [`ServiceDeregistrationForm`] will return [`SocketAddr`] for deregistration.
///
#[derive(Constructor)]
pub struct ServiceDeregistrationForm(oneshot::Sender<SocketAddr>);

impl ServiceDeregistrationForm {
    pub(crate) fn send(self, registration_id: SocketAddr) -> Result<(), SocketAddr> {
        self.0.send(registration_id)
    }
}
