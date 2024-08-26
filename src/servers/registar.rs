//! Registar. Registers Services for Health Check.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Weak};

use futures::channel::oneshot;
use tokio::sync::Mutex;
use torrust_tracker_services::registration::{Registration, ServiceRegistrationForm};

/// The [`ServiceRegistry`] contains each unique [`ServiceRegistration`] by it's [`SocketAddr`].
pub type ServiceRegistry = Mutex<HashMap<SocketAddr, Registration>>;

/// The [`Registar`] manages the [`ServiceRegistry`].
#[derive(Debug)]
pub struct Registar {
    me: Weak<Self>,
    registry: ServiceRegistry,
}

impl Registar {
    pub fn new(registry: ServiceRegistry) -> Arc<Self> {
        Arc::new_cyclic(|me| Self {
            me: me.clone(),
            registry,
        })
    }

    fn me(&self) -> Arc<Self> {
        self.me.upgrade().unwrap()
    }

    /// Registers a Service
    #[must_use]
    pub fn give_form(&self) -> ServiceRegistrationForm {
        let (tx, rx) = oneshot::channel::<Registration>();
        let register = self.me();
        tokio::spawn(async move {
            register.insert(rx).await;
        });
        ServiceRegistrationForm::new(tx)
    }

    /// Inserts a listing into the registry.
    async fn insert(&self, rx: oneshot::Receiver<Registration>) {
        tracing::debug!("Waiting for the started service to send registration data ...");

        let service_registration = rx
            .await
            .expect("it should receive the service registration from the started service");

        let mut mutex = self.registry.lock().await;

        mutex.insert(service_registration.local_addr(), service_registration);
    }

    /// Returns the [`ServiceRegistry`] of services
    #[must_use]
    pub fn entries(&self) -> &ServiceRegistry {
        &self.registry
    }
}
