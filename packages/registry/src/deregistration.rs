use std::marker::PhantomData;
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
pub struct Form<Registry, CheckSuccess, CheckError> {
    registry: Arc<Registry>,
    _check_success: PhantomData<CheckSuccess>,
    _check_error: PhantomData<CheckError>,
}

impl<Registry, CheckSuccess, CheckError> Form<Registry, CheckSuccess, CheckError> {
    pub fn new(registry: Arc<Registry>) -> Self {
        Self {
            registry,
            _check_success: PhantomData,
            _check_error: PhantomData,
        }
    }
}

impl<'b, Registry, CheckSuccess, CheckError>
    ServiceDeregistrationForm<'b, Deregistration<<Registry as ServiceRegistry<CheckSuccess, CheckError>>::Key>>
    for Form<Registry, CheckSuccess, CheckError>
where
    Registry: ServiceRegistry<CheckSuccess, CheckError>,
    <Registry as ServiceRegistry<CheckSuccess, CheckError>>::Key: std::fmt::Debug + std::fmt::Display,

    CheckSuccess: 'b,
    CheckError: 'b,
{
    type DeReg = Deregistration<<Registry as ServiceRegistry<CheckSuccess, CheckError>>::Key>;
    type Error = Error<<Registry as ServiceRegistry<CheckSuccess, CheckError>>::Key>;

    fn deregister(
        self,
        deregistration: Deregistration<<Registry as ServiceRegistry<CheckSuccess, CheckError>>::Key>,
    ) -> Result<BoxFuture<'b, Result<(), Box<dyn std::error::Error + Send>>>, Self::Error> {
        let key = deregistration.0;

        match self.registry.deregister(key) {
            Ok(()) => Ok(std::future::ready(Ok(())).boxed()),
            Err(key) => Err(Error::FailedToDeregister(key)),
        }
    }
}
