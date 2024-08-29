use std::marker::PhantomData;
use std::sync::Arc;

use futures::future::BoxFuture;
use futures::FutureExt as _;
use thiserror::Error;
use torrust_tracker_services::{Registration, ServiceRegistrationForm};

use crate::deregistration::Deregistration;
use crate::service_registry::{Checker, ServiceRegistry};

#[derive(Debug, Error)]
pub enum Error<CheckSuccess, CheckError> {
    #[error("Failed to get check from registration {0}")]
    FailedToGetCheck(Box<dyn Registration<Success = CheckSuccess, Error = CheckError>>),
    #[error("Failed register check {0}")]
    FailedToRegister(Checker<CheckSuccess, CheckError>),
}

#[derive(Debug)]
pub struct Form<Registry, CheckSuccess, CheckError> {
    registry: Arc<Registry>,
    _check_success: PhantomData<CheckSuccess>,
    _check_error: PhantomData<CheckError>,
}

impl<Registry, CheckSuccess, CheckError> Form<Registry, CheckSuccess, CheckError> {
    pub(crate) fn new(registry: Arc<Registry>) -> Self {
        Self {
            registry,
            _check_success: PhantomData,
            _check_error: PhantomData,
        }
    }
}

impl<'b, Registry, CheckSuccess, CheckError> ServiceRegistrationForm<'b, CheckSuccess, CheckError>
    for Form<Registry, CheckSuccess, CheckError>
where
    Registry: ServiceRegistry<CheckSuccess, CheckError>,
    <Registry as ServiceRegistry<CheckSuccess, CheckError>>::Key: std::fmt::Debug + Send + 'b,
    CheckSuccess: std::fmt::Debug + std::fmt::Display + 'static,
    CheckError: std::fmt::Debug + std::fmt::Display + 'static,
{
    type DeReg = Deregistration<<Registry as ServiceRegistry<CheckSuccess, CheckError>>::Key>;
    type Error = Error<CheckSuccess, CheckError>;

    fn register(
        self,
        mut registration: Box<dyn Registration<Success = CheckSuccess, Error = CheckError>>,
    ) -> Result<BoxFuture<'b, Result<Option<Self::DeReg>, Self::Error>>, Self::Error> {
        let Some(check) = registration.check() else {
            return Err(Error::FailedToGetCheck(registration));
        };

        let maybe_de_reg = self
            .registry
            .register(check)
            .map_err(|e| Error::FailedToRegister(e))?
            .map(Deregistration::new);

        Ok(std::future::ready(Ok(maybe_de_reg)).boxed())
    }
}
