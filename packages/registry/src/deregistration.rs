use std::sync::Arc;

use derive_more::derive::{Constructor, Display};
use futures::future::BoxFuture;
use futures::FutureExt as _;
use thiserror::Error;
use torrust_tracker_services::{ServiceDeregistration, ServiceDeregistrationForm};

use crate::service_registry::ServiceRegistry;

#[derive(Display, Constructor, Debug)]
pub struct Deregistration<Key>(Key);

impl<Key> ServiceDeregistration for Deregistration<Key>
where
    Key: std::fmt::Debug,
{
    type Key = Key;
}

#[derive(Debug, Error)]
pub enum Error<K> {
    #[error("Failed to send successful response {0}")]
    FailedToDeregister(K),
}

#[derive(Debug)]
pub struct Form<Registry> {
    registry: Arc<Registry>,
}

impl<Registry> Form<Registry> {
    pub fn new(registry: Arc<Registry>) -> Self {
        Self { registry }
    }
}

impl<'b, Registry> ServiceDeregistrationForm<'b, Deregistration<<Registry as ServiceRegistry>::Key>> for Form<Registry>
where
    Registry: ServiceRegistry,
    <Registry as ServiceRegistry>::Key: std::fmt::Debug + std::fmt::Display,
{
    type DeReg = Deregistration<<Registry as ServiceRegistry>::Key>;
    type Error = Error<<Registry as ServiceRegistry>::Key>;

    fn deregister(
        self,
        deregistration: Deregistration<<Registry as ServiceRegistry>::Key>,
    ) -> Result<BoxFuture<'b, Result<(), Box<dyn std::error::Error + Send>>>, Self::Error> {
        let key = deregistration.0;

        match self.registry.deregister(key) {
            Ok(()) => Ok(std::future::ready(Ok(())).boxed()),
            Err(key) => Err(Error::FailedToDeregister(key)),
        }
    }
}
