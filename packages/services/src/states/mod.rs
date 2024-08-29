//! Service States Module
//!
//! This module manages the different states of a service's lifecycle, including starting, running, stopping, and halting.
//! Each state is represented by a distinct structure and associated functionality to transition between states.
//!
//! The states are organized into the following submodules:
//!
//! - [`stopped`]: Contains the [`Stopped`] struct and related functionality for services that are not currently running.
//! - [`starting`]: Contains the [`Starting`] future that represents a service in the process of being started.
//! - [`running`]: Contains the [`Running`] struct and related functionality for services that are currently running.
//! - [`halting`]: Contains the [`Halting`] future that represents a service in the process of being stopped.
//!
//! The primary structures and their transitions are as follows:
//!
//! - [`Stopped`]: Represents a service that is not currently running. Provides the [`start`][Stopped::start] method to initiate the service.
//! - [`Starting`]: Represents a service that is in the process of being started. Upon completion, it returns the [`Running`] structure.
//! - [`Running`]: Represents a service that is currently running. Provides the [`stop`][Running::stop] method to shut down the service and the [`deregister_and_stop`][Running::deregister_and_stop] method to deregister and stop the service.
//! - [`Halting`]: Represents a service that is in the process of being stopped. Upon completion, it returns the [`Stopped`] structure.

mod halting;
mod running;
mod starting;
mod stopped;

pub use halting::Halting;
pub use running::Running;
pub use starting::Starting;
pub use stopped::Stopped;
