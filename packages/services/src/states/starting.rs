//! Starting Service - A service that is in the process of being started.
//!
//! This module contains the [`Starting`] future that represents a service in the process of being started.
//! The [`Starting`] state is responsible for transitioning a service from the [`Stopped`][super::Stopped] state to the [`Running`] state.
//!
//! The primary functionality provided by this module includes:
//!
//! - The [`Starting`] future: Represents a service that is in the process of being started. Upon completion, it returns the [`Running`] structure.
//! - The [`Error`] enum: Defines possible errors that can occur during the starting process, including service-specific errors and registration errors.

use std::future::Future;
use std::marker::PhantomData;

use futures::future::BoxFuture;
use futures::FutureExt as _;
use pin_project::pin_project;
use thiserror::Error;

use super::running::Running;
use crate::registration::{Registration, ServiceRegistrationForm};
use crate::service::Service;

#[derive(Debug, Error)]
pub enum Error<ServiceError, RegError> {
    Service(ServiceError),
    Registration(RegError),
}

// A future that represents a service in the process of being started.
///
/// This future handles the transition from the [`Stopped`] state to the [`Running`] state.
/// Upon successful completion, it returns the [`Running`] structure.
#[pin_project]
pub struct Starting<'a, 'b, S, Reg, RegError, Form, CheckSuccess, CheckError>
where
    S: Service,
    Form: ServiceRegistrationForm<'b, CheckSuccess, CheckError, Error = RegError>,
{
    #[pin]
    pub(super) starting_service: BoxFuture<'a, Result<(S, Reg), S::Error>>,

    #[allow(clippy::type_complexity)]
    #[pin]
    pub(super) registration:
        Option<BoxFuture<'b, Result<Option<<Form as ServiceRegistrationForm<'b, CheckSuccess, CheckError>>::DeReg>, RegError>>>,

    pub(super) form: Option<Form>,
    pub(super) running_service: Option<S>,
    pub(super) _check_success: PhantomData<CheckSuccess>,
    pub(super) _check_error: PhantomData<CheckError>,
}

impl<'a, 'b, S, Reg, RegError, Form, CheckSuccess, CheckError> Future
    for Starting<'a, 'b, S, Reg, RegError, Form, CheckSuccess, CheckError>
where
    S: Service,
    Reg: Registration<Success = CheckSuccess, Error = CheckError>,
    Form: ServiceRegistrationForm<'b, CheckSuccess, CheckError, Error = RegError>,
{
    type Output =
        Result<Running<S, <Form as ServiceRegistrationForm<'b, CheckSuccess, CheckError>>::DeReg>, Error<S::Error, RegError>>;

    fn poll(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        if self.running_service.is_none() {
            return match self.starting_service.poll_unpin(cx) {
                std::task::Poll::Pending => std::task::Poll::Pending,
                std::task::Poll::Ready(Err(e)) => std::task::Poll::Ready(Err(Error::Service(e))),

                std::task::Poll::Ready(Ok((running_service, registration))) => {
                    let form = self.form.take().expect("it should have a form");

                    match form.register(Box::new(registration)) {
                        Ok(registration) => {
                            self.running_service = Some(running_service);
                            self.registration = Some(registration);
                            cx.waker().wake_by_ref();
                            std::task::Poll::Pending
                        }
                        Err(e) => std::task::Poll::Ready(Err(Error::Registration(e))),
                    }
                }
            };
        };

        let Some(ref mut registration) = self.registration else {
            unreachable!("it should have a registration")
        };

        let deregistration = match registration.poll_unpin(cx) {
            std::task::Poll::Ready(Ok(deregistration)) => deregistration,
            std::task::Poll::Ready(Err(e)) => return std::task::Poll::Ready(Err(Error::Registration(e))),
            std::task::Poll::Pending => return std::task::Poll::Pending,
        };

        let Some(running_service) = self.running_service.take() else {
            unreachable!("it should have a running service")
        };

        let running = Running {
            running_service,
            deregistration,
        };

        std::task::Poll::Ready(Ok(running))
    }
}
