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
pub struct Form<Registry> {
    registry: Arc<Registry>,
}

impl<Registry> Form<Registry> {
    pub(crate) fn new(registry: Arc<Registry>) -> Self {
        Self { registry }
    }
}

impl<'b, Registry>
    ServiceRegistrationForm<'b, <Registry as ServiceRegistry>::CheckSuccess, <Registry as ServiceRegistry>::CheckError>
    for Form<Registry>
where
    Registry: ServiceRegistry,
    <Registry as ServiceRegistry>::Key: std::fmt::Debug + Send + 'b,
    <Registry as ServiceRegistry>::CheckSuccess: std::fmt::Debug + 'static,
    <Registry as ServiceRegistry>::CheckError: std::error::Error + 'static,
{
    type DeReg = Deregistration<<Registry as ServiceRegistry>::Key>;
    type Error = Error<<Registry as ServiceRegistry>::CheckSuccess, <Registry as ServiceRegistry>::CheckError>;

    fn register(
        self,
        mut registration: Box<
            dyn Registration<
                Success = <Registry as ServiceRegistry>::CheckSuccess,
                Error = <Registry as ServiceRegistry>::CheckError,
            >,
        >,
    ) -> Result<BoxFuture<'b, Result<Option<Self::DeReg>, Self::Error>>, Self::Error> {
        let Some(check) = registration.check() else {
            return Err(Error::FailedToGetCheck(registration));
        };

        let maybe_de_reg = self
            .registry
            .register(check)
            .map_err(Error::FailedToRegister)?
            .map(Deregistration::new);

        Ok(std::future::ready(Ok(maybe_de_reg)).boxed())
    }
}
