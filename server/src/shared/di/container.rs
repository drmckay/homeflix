//! Dependency Injection Container
//!
//! Provides service registration and resolution with support for
//! different service lifetimes (transient, singleton).

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Arc;

use crate::shared::di::{ServiceLifetime, DIError, DIResult};

/// Factory function type for creating service instances
pub type ServiceFactory<T> = Box<dyn Fn() -> T + Send + Sync>;

/// Async factory function type for creating service instances
pub type AsyncServiceFactory<T> = Box<dyn Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = T> + Send>> + Send + Sync>;

/// Service container for dependency injection
///
/// The container manages service instances based on their lifetime:
/// - **Transient**: New instance created for each request
/// - **Singleton**: Single instance created and reused
pub struct ServiceContainer {
    /// Transient services (factories that create new instances)
    transient_services: HashMap<TypeId, Box<dyn Any + Send + Sync>>,

    /// Singleton services (shared instances wrapped in Arc)
    singleton_services: HashMap<TypeId, Arc<dyn Any + Send + Sync>>,

    /// Singleton service factories (to defer creation until first use)
    singleton_factories: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl ServiceContainer {
    /// Create a new empty service container
    pub fn new() -> Self {
        Self {
            transient_services: HashMap::new(),
            singleton_services: HashMap::new(),
            singleton_factories: HashMap::new(),
        }
    }

    /// Register a service instance with specified lifetime
    ///
    /// # Arguments
    /// * `instance` - The service instance to register
    /// * `lifetime` - The lifetime of service (Transient or Singleton)
    pub fn register<T: 'static + Clone + Send + Sync>(&mut self, instance: T, lifetime: ServiceLifetime) {
        let type_id = TypeId::of::<T>();

        match lifetime {
            ServiceLifetime::Transient => {
                // For transient, we store instance as Any
                self.transient_services.insert(type_id, Box::new(instance));
            }
            ServiceLifetime::Singleton => {
                // For singleton, wrap in Arc for shared access
                self.singleton_services.insert(type_id, Arc::new(instance));
            }
        }
    }

    /// Register a service factory with specified lifetime
    ///
    /// # Arguments
    /// * `factory` - Factory function that creates service instances
    /// * `lifetime` - The lifetime of service (Transient or Singleton)
    pub fn register_factory<T: 'static + Clone + Send + Sync, F>(&mut self, factory: F, lifetime: ServiceLifetime)
    where
        F: Fn() -> T + 'static + Send + Sync,
    {
        let type_id = TypeId::of::<T>();
        let factory_obj: Box<dyn Fn() -> T + Send + Sync> = Box::new(factory);

        match lifetime {
            ServiceLifetime::Transient => {
                // Store factory for transient services
                self.transient_services.insert(type_id, Box::new(factory_obj));
            }
            ServiceLifetime::Singleton => {
                // Store factory for deferred creation
                self.singleton_factories.insert(type_id, Box::new(factory_obj));
            }
        }
    }

    /// Register an async service factory
    ///
    /// # Arguments
    /// * `factory` - Async factory function that creates service instances
    /// * `lifetime` - The lifetime of service (Transient or Singleton)
    #[allow(dead_code)]
    pub fn register_async_factory<T: 'static + Clone + Send + Sync, F, Fut>(&mut self, factory: F, lifetime: ServiceLifetime)
    where
        F: Fn() -> Fut + 'static + Send + Sync,
        Fut: std::future::Future<Output = T> + Send + 'static,
    {
        let type_id = TypeId::of::<T>();
        let factory_obj: Box<dyn Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = T> + Send>> + Send + Sync> = 
            Box::new(move || Box::pin(factory()));

        match lifetime {
            ServiceLifetime::Transient => {
                // Store async factory for transient services
                self.transient_services.insert(type_id, Box::new(factory_obj));
            }
            ServiceLifetime::Singleton => {
                // Store async factory for deferred creation
                self.singleton_factories.insert(type_id, Box::new(factory_obj));
            }
        }
    }

    /// Resolve a service from the container
    ///
    /// # Arguments
    /// * `T` - The type of service to resolve
    ///
    /// # Returns
    /// * `Result<Arc<T>, DIError>` - The resolved service or an error
    pub fn resolve<T: 'static + Clone + Send + Sync>(&self) -> DIResult<Arc<T>> {
        let type_id = TypeId::of::<T>();

        // Check singleton services first
        if let Some(instance) = self.singleton_services.get(&type_id) {
            return instance
                .clone()
                .downcast::<T>()
                .map_err(|_| DIError::DowncastError);
        }

        // Check singleton factories (deferred creation)
        if let Some(factory) = self.singleton_factories.get(&type_id) {
            // Try to downcast as a factory
            if let Some(factory) = factory.downcast_ref::<Box<dyn Fn() -> T + Send + Sync>>() {
                let instance = factory();
                let arc_instance = Arc::new(instance);
                return Ok(arc_instance);
            }
        }

        // Check transient services
        if let Some(service) = self.transient_services.get(&type_id) {
            // Try to downcast as a factory first
            if let Some(factory) = service.downcast_ref::<Box<dyn Fn() -> T + Send + Sync>>() {
                let instance = factory();
                return Ok(Arc::new(instance));
            }

            // Try to downcast as a cloneable instance
            if let Some(instance) = service.downcast_ref::<T>() {
                return Ok(Arc::new(instance.clone()));
            }
        }

        Err(DIError::ServiceNotFound(std::any::type_name::<T>().to_string()))
    }

    /// Resolve a required service, panicking if not found
    ///
    /// # Arguments
    /// * `T` - The type of service to resolve
    ///
    /// # Returns
    /// * `Arc<T>` - The resolved service
    ///
    /// # Panics
    /// Panics if service is not registered
    pub fn resolve_required<T: 'static + Clone + std::marker::Sync + Send + Sync>(&self) -> Arc<T> {
        self.resolve().expect(&format!(
            "Required service '{}' not found in container",
            std::any::type_name::<T>()
        ))
    }

    /// Check if a service is registered
    ///
    /// # Arguments
    /// * `T` - The type of service to check
    ///
    /// # Returns
    /// * `bool` - True if service is registered
    pub fn has<T: 'static>(&self) -> bool {
        let type_id = TypeId::of::<T>();
        self.singleton_services.contains_key(&type_id)
            || self.singleton_factories.contains_key(&type_id)
            || self.transient_services.contains_key(&type_id)
    }

    /// Get number of registered services
    pub fn service_count(&self) -> usize {
        self.singleton_services.len()
            + self.singleton_factories.len()
            + self.transient_services.len()
    }

    /// Clear all registered services
    pub fn clear(&mut self) {
        self.singleton_services.clear();
        self.singleton_factories.clear();
        self.transient_services.clear();
    }
}

impl Default for ServiceContainer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    struct TestService {
        value: String,
    }

    impl TestService {
        fn new(value: &str) -> Self {
            Self {
                value: value.to_string(),
            }
        }
    }

    #[test]
    fn test_singleton_registration() {
        let mut container = ServiceContainer::new();
        let service = TestService::new("singleton");

        container.register(service, ServiceLifetime::Singleton);

        let resolved1 = container.resolve::<TestService>().unwrap();
        let resolved2 = container.resolve::<TestService>().unwrap();

        // Both should point to same instance
        assert!(Arc::ptr_eq(&resolved1, &resolved2));
        assert_eq!(resolved1.as_ref().value, "singleton");
    }

    #[test]
    fn test_transient_registration() {
        let mut container = ServiceContainer::new();
        let service = TestService::new("transient");

        container.register(service, ServiceLifetime::Transient);

        let resolved1 = container.resolve::<TestService>().unwrap();
        let resolved2 = container.resolve::<TestService>().unwrap();

        // Both should have same value (cloned)
        assert_eq!(resolved1.as_ref().value, "transient");
        assert_eq!(resolved2.as_ref().value, "transient");
    }

    #[test]
    fn test_factory_registration() {
        let mut container = ServiceContainer::new();

        container.register_factory(
            || TestService::new("factory"),
            ServiceLifetime::Transient,
        );

        let resolved = container.resolve::<TestService>().unwrap();
        assert_eq!(resolved.as_ref().value, "factory");
    }

    #[test]
    fn test_service_not_found() {
        let container = ServiceContainer::new();
        let result = container.resolve::<TestService>();

        assert!(result.is_err());
        assert!(matches!(result, Err(DIError::ServiceNotFound(_))));
    }

    #[test]
    fn test_has_service() {
        let mut container = ServiceContainer::new();
        let service = TestService::new("test");

        assert!(!container.has::<TestService>());

        container.register(service, ServiceLifetime::Singleton);

        assert!(container.has::<TestService>());
    }
}
