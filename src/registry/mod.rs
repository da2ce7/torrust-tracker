use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use key_generator::KeyGenerator;
use torrust_tracker_registry::service_registry::{Checker, ServiceRegistry};

pub mod key_generator;

#[derive(Debug, Clone)]
pub struct Registry {
    services: Arc<Mutex<HashMap<usize, Checker<Box<dyn CheckSuccess>, Box<dyn CheckError>>>>>,
    id_generator: Arc<KeyGenerator>,
}

pub trait CheckSuccess: std::fmt::Debug + std::fmt::Display {}
pub trait CheckError: std::error::Error {}

impl ServiceRegistry for Registry {
    type Key = usize;
    type CheckSuccess = Box<dyn CheckSuccess>;
    type CheckError = Box<dyn CheckError>;

    fn new() -> Arc<Self> {
        Arc::new(Self {
            services: Arc::new(Mutex::new(HashMap::new())),
            id_generator: Arc::new(KeyGenerator::default()),
        })
    }

    fn me(&self) -> std::sync::Weak<Self> {
        Arc::downgrade(&Arc::new(self.clone()))
    }

    fn register(
        self: Arc<Self>,
        value: Checker<Self::CheckSuccess, Self::CheckError>,
    ) -> Result<Option<Self::Key>, Checker<Self::CheckSuccess, Self::CheckError>> {
        let mut services = self.services.lock().unwrap();
        let key = self.id_generator.next_id().ok_or(value)?;
        services.insert(key, value);
        Ok(Some(key))
    }

    fn deregister(self: Arc<Self>, key: Self::Key) -> Result<(), Self::Key> {
        let mut services = self.services.lock().unwrap();
        services.remove(&key).ok_or(key)?;
        Ok(())
    }
}
