//! Service Registry with Dependency Graph Validation
//!
//! Extends the ServiceContainer with dependency tracking and
//! circular dependency detection.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::shared::di::{ServiceContainer, ServiceLifetime, DIError, DIResult};

/// Service registry with dependency graph validation
///
/// The registry extends the basic container with:
/// - Dependency tracking
/// - Circular dependency detection
/// - Dependency graph validation
pub struct ServiceRegistry {
    /// Underlying service container
    container: ServiceContainer,

    /// Dependency graph: service name -> list of dependencies
    dependency_graph: HashMap<String, Vec<String>>,

    /// Reverse dependency graph: service name -> list of dependents
    reverse_dependency_graph: HashMap<String, Vec<String>>,
}

impl ServiceRegistry {
    /// Create a new service registry
    pub fn new() -> Self {
        Self {
            container: ServiceContainer::new(),
            dependency_graph: HashMap::new(),
            reverse_dependency_graph: HashMap::new(),
        }
    }

    /// Register a service with the specified lifetime
    ///
    /// # Arguments
    /// * `instance` - The service instance to register
    /// * `lifetime` - The lifetime of the service
    pub fn register<T: 'static + Clone + Send + Sync>(&mut self, instance: T, lifetime: ServiceLifetime) {
        let type_name = std::any::type_name::<T>().to_string();
        self.container.register(instance, lifetime);
        self.dependency_graph.insert(type_name.clone(), Vec::new());
        self.reverse_dependency_graph.insert(type_name, Vec::new());
    }

    /// Register a service with explicit dependencies
    ///
    /// # Arguments
    /// * `instance` - The service instance to register
    /// * `lifetime` - The lifetime of the service
    /// * `dependencies` - List of type names this service depends on
    pub fn register_with_dependencies<T: 'static + Clone + Send + Sync>(
        &mut self,
        instance: T,
        lifetime: ServiceLifetime,
        dependencies: &[&str],
    ) {
        let type_name = std::any::type_name::<T>().to_string();
        self.container.register(instance, lifetime);

        // Update dependency graph
        let deps: Vec<String> = dependencies.iter().map(|s| s.to_string()).collect();
        self.dependency_graph.insert(type_name.clone(), deps);

        // Update reverse dependency graph
        for dep in dependencies {
            self.reverse_dependency_graph
                .entry(dep.to_string())
                .or_insert_with(Vec::new)
                .push(type_name.clone());
        }
    }

    /// Register a service factory with the specified lifetime
    ///
    /// # Arguments
    /// * `factory` - Factory function that creates service instances
    /// * `lifetime` - The lifetime of the service
    pub fn register_factory<T: 'static + Clone + Send + Sync, F>(&mut self, factory: F, lifetime: ServiceLifetime)
    where
        F: Fn() -> T + 'static + Send + Sync,
    {
        let type_name = std::any::type_name::<T>().to_string();
        self.container.register_factory(factory, lifetime);
        self.dependency_graph.insert(type_name.clone(), Vec::new());
        self.reverse_dependency_graph.insert(type_name, Vec::new());
    }

    /// Register a service factory with explicit dependencies
    ///
    /// # Arguments
    /// * `factory` - Factory function that creates service instances
    /// * `lifetime` - The lifetime of the service
    /// * `dependencies` - List of type names this service depends on
    pub fn register_factory_with_dependencies<T: 'static + Clone + Send + Sync, F>(
        &mut self,
        factory: F,
        lifetime: ServiceLifetime,
        dependencies: &[&str],
    ) where
        F: Fn() -> T + 'static + Send + Sync,
    {
        let type_name = std::any::type_name::<T>().to_string();
        self.container.register_factory(factory, lifetime);

        // Update dependency graph
        let deps: Vec<String> = dependencies.iter().map(|s| s.to_string()).collect();
        self.dependency_graph.insert(type_name.clone(), deps);

        // Update reverse dependency graph
        for dep in dependencies {
            self.reverse_dependency_graph
                .entry(dep.to_string())
                .or_insert_with(Vec::new)
                .push(type_name.clone());
        }
    }

    /// Validate the dependency graph for circular dependencies
    ///
    /// # Returns
    /// * `Result<(), DIError>` - Ok if no circular dependencies, error otherwise
    pub fn validate_dependencies(&self) -> DIResult<()> {
        let mut visited = HashSet::new();
        let mut recursion_stack = HashSet::new();

        for node in self.dependency_graph.keys() {
            if !visited.contains(node) {
                self.dfs(node, &mut visited, &mut recursion_stack)?;
            }
        }

        Ok(())
    }

    /// Depth-first search for circular dependency detection
    fn dfs(
        &self,
        node: &str,
        visited: &mut HashSet<String>,
        recursion_stack: &mut HashSet<String>,
    ) -> DIResult<()> {
        visited.insert(node.to_string());
        recursion_stack.insert(node.to_string());

        if let Some(dependencies) = self.dependency_graph.get(node) {
            for dep in dependencies {
                if !visited.contains(dep) {
                    self.dfs(dep, visited, recursion_stack)?;
                } else if recursion_stack.contains(dep) {
                    // Found a circular dependency
                    let cycle = self.extract_cycle(node, dep)?;
                    return Err(DIError::CircularDependency(cycle));
                }
            }
        }

        recursion_stack.remove(node);
        Ok(())
    }

    /// Extract the circular dependency cycle for error reporting
    fn extract_cycle(&self, start: &str, end: &str) -> DIResult<String> {
        let mut cycle = vec![end.to_string()];
        let mut current = start;

        cycle.push(current.to_string());

        // Try to trace back the cycle
        while let Some(deps) = self.dependency_graph.get(current) {
            if deps.is_empty() {
                break;
            }
            let next = &deps[0];
            if cycle.contains(next) {
                cycle.push(next.to_string());
                break;
            }
            cycle.push(next.to_string());
            current = next;
        }

        Ok(format!("Circular dependency detected: {}", cycle.join(" -> ")))
    }

    /// Get all dependencies of a service
    ///
    /// # Arguments
    /// * `type_name` - The type name of the service
    ///
    /// # Returns
    /// * `Option<&Vec<String>>` - The list of dependencies if the service exists
    pub fn get_dependencies(&self, type_name: &str) -> Option<&Vec<String>> {
        self.dependency_graph.get(type_name)
    }

    /// Get all services that depend on a given service
    ///
    /// # Arguments
    /// * `type_name` - The type name of the service
    ///
    /// # Returns
    /// * `Option<&Vec<String>>` - The list of dependents if the service exists
    pub fn get_dependents(&self, type_name: &str) -> Option<&Vec<String>> {
        self.reverse_dependency_graph.get(type_name)
    }

    /// Resolve a service from the container
    ///
    /// # Arguments
    /// * `T` - The type of service to resolve
    ///
    /// # Returns
    /// * `Result<Arc<T>, DIError>` - The resolved service or an error
    pub fn resolve<T: 'static + Clone + Send + Sync>(&self) -> DIResult<Arc<T>> {
        self.container.resolve()
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
    /// Panics if the service is not registered
    pub fn resolve_required<T: 'static + Clone + Send + Sync>(&self) -> Arc<T> {
        self.container.resolve_required()
    }

    /// Check if a service is registered
    ///
    /// # Arguments
    /// * `T` - The type of service to check
    ///
    /// # Returns
    /// * `bool` - True if the service is registered
    pub fn has<T: 'static>(&self) -> bool {
        self.container.has::<T>()
    }

    /// Get the number of registered services
    pub fn service_count(&self) -> usize {
        self.container.service_count()
    }

    /// Get a reference to the underlying container
    pub fn container(&self) -> &ServiceContainer {
        &self.container
    }

    /// Get a mutable reference to the underlying container
    pub fn container_mut(&mut self) -> &mut ServiceContainer {
        &mut self.container
    }

    /// Clear all registered services and dependency graphs
    pub fn clear(&mut self) {
        self.container.clear();
        self.dependency_graph.clear();
        self.reverse_dependency_graph.clear();
    }
}

impl Default for ServiceRegistry {
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

    #[derive(Debug, Clone, PartialEq)]
    struct OtherService {
        value: String,
    }

    impl OtherService {
        fn new(value: &str) -> Self {
            Self {
                value: value.to_string(),
            }
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    struct ServiceA { value: String }
    #[derive(Debug, Clone, PartialEq)]
    struct ServiceB { value: String }
    #[derive(Debug, Clone, PartialEq)]
    struct ServiceC { value: String }

    #[test]
    fn test_registry_registration() {
        let mut registry = ServiceRegistry::new();
        let service = TestService::new("test");

        registry.register(service, ServiceLifetime::Singleton);

        assert!(registry.has::<TestService>());
        assert_eq!(registry.service_count(), 1);
    }

    #[test]
    fn test_dependency_tracking() {
        let mut registry = ServiceRegistry::new();
        let service = TestService::new("test");

        registry.register_with_dependencies(
            service,
            ServiceLifetime::Singleton,
            &["SomeOtherService"],
        );

        let type_name = std::any::type_name::<TestService>();
        let deps = registry.get_dependencies(type_name);
        assert!(deps.is_some());
        assert_eq!(deps.unwrap(), &vec!["SomeOtherService".to_string()]);
    }

    #[test]
    fn test_circular_dependency_detection() {
        let mut registry = ServiceRegistry::new();

        // Register service A that depends on B
        let service_a = ServiceA { value: "a".into() };
        let type_b = std::any::type_name::<ServiceB>();
        registry.register_with_dependencies(
            service_a,
            ServiceLifetime::Singleton,
            &[type_b],
        );

        // Register service B that depends on A (circular!)
        let service_b = ServiceB { value: "b".into() };
        let type_a = std::any::type_name::<ServiceA>();
        registry.register_with_dependencies(
            service_b,
            ServiceLifetime::Singleton,
            &[type_a],
        );

        // Should detect circular dependency
        let result = registry.validate_dependencies();
        assert!(result.is_err());
        assert!(matches!(result, Err(DIError::CircularDependency(_))));
    }

    #[test]
    fn test_no_circular_dependency() {
        let mut registry = ServiceRegistry::new();

        let type_b = std::any::type_name::<ServiceB>();
        let type_c = std::any::type_name::<ServiceC>();

        // Register service A that depends on B
        let service_a = ServiceA { value: "a".into() };
        registry.register_with_dependencies(
            service_a,
            ServiceLifetime::Singleton,
            &[type_b],
        );

        // Register service B that depends on C
        let service_b = ServiceB { value: "b".into() };
        registry.register_with_dependencies(
            service_b,
            ServiceLifetime::Singleton,
            &[type_c],
        );

        // Register service C with no dependencies
        let service_c = ServiceC { value: "c".into() };
        registry.register(service_c, ServiceLifetime::Singleton);

        // Should pass validation
        let result = registry.validate_dependencies();
        assert!(result.is_ok());
    }

    #[test]
    fn test_reverse_dependencies() {
        let mut registry = ServiceRegistry::new();

        let service_a = TestService::new("a");
        registry.register_with_dependencies(
            service_a,
            ServiceLifetime::Singleton,
            &["ServiceB"],
        );

        let type_name = std::any::type_name::<TestService>().to_string();
        let dependents = registry.get_dependents("ServiceB");
        assert!(dependents.is_some());
        assert_eq!(dependents.unwrap(), &vec![type_name]);
    }
}
