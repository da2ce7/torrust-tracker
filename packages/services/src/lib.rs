//! Services Management Package
//!
//! This package provides core functionality for managing the lifecycle of services,
//! including starting, running, stopping, and registering services for health checks.
//!
//! The package is organized into several modules:
//!
//! - The [`service`] module: Defines the [`Service`] trait, which should be implemented by other libraries that wish to use this service management library.
//! - The [`registration`] module: Defines the [`ServiceCheck`] trait and other traits related to the registration and deregistration of services.
//! - The [`states`] module: Manages the different states of a service's lifecycle, including starting, running, stopping, and halting. For more details, refer to the [`states`] module documentation.
//!
//! The primary structures and their transitions are as follows:
//!
//! - [`Stopped`]: Represents a service that is not currently running. Provides the [`start`][Stopped::start] method to initiate the service.
//! - [`Starting`]: A future that represents a service in the process of being started. Upon completion, it returns the [`Running`] structure.
//! - [`Running`]: Represents a service that is currently running. Provides the [`stop`][Running::stop] method to shut down the service and the [`deregister_and_stop`][Running::deregister_and_stop] method to deregister and stop the service. Additionally, the [`Running`] structure implements [`Stream<Item = Error>`][futures::Stream], providing a stream of errors from the running service.
//! - [`Halting`]: A future that represents a service in the process of being stopped. Upon completion, it returns the [`Stopped`] structure.
//!
//! For more detailed information on the service states and their transitions, please refer to the [`states`] module documentation.
//!

mod registration;
mod service;
mod states;

pub use registration::{Registration, ServiceCheck, ServiceDeregistration, ServiceDeregistrationForm, ServiceRegistrationForm};
pub use service::Service;
pub use states::{Halting, Running, Starting, Stopped};
