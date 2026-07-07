//! # Chronos Service Container
//!
//! The Service Container is the canonical dependency resolution mechanism for the PCOS.
//! It owns service instances and provides type-safe dependency resolution.
//!
//! It is NOT responsible for lifecycle management, configuration, or plugin loading.
//! It acts strictly as an inversion-of-control (IoC) container mapping types to singletons.

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Errors that can occur during service resolution or registration.
#[derive(Debug, thiserror::Error)]
pub enum ContainerError {
    #[error("Service of type '{0}' is already registered")]
    AlreadyRegistered(&'static str),
    #[error("Service of type '{0}' not found in the container")]
    NotFound(&'static str),
}

/// A thread-safe Dependency Injection container for resolving PCOS subsystems.
#[derive(Clone)]
pub struct ServiceContainer {
    /// Maps a unique TypeId to a boxed `Any` instance.
    /// By storing `Box<dyn Any + Send + Sync>`, consumers can register `Arc<dyn Trait>` directly.
    services: Arc<RwLock<HashMap<TypeId, Box<dyn Any + Send + Sync>>>>,
}

impl ServiceContainer {
    /// Creates a new, empty ServiceContainer.
    pub fn new() -> Self {
        Self {
            services: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Registers a service instance in the container.
    /// 
    /// To register by interface, the type `T` should be an `Arc<dyn Trait>`.
    /// 
    /// # Example
    /// ```ignore
    /// let my_service: Arc<dyn MyTrait + Send + Sync> = Arc::new(MyImplementation);
    /// container.register(my_service).await.unwrap();
    /// ```
    pub async fn register<T: Any + Send + Sync>(&self, instance: T) -> Result<(), ContainerError> {
        let type_id = TypeId::of::<T>();
        let mut lock = self.services.write().await;
        
        if lock.contains_key(&type_id) {
            return Err(ContainerError::AlreadyRegistered(std::any::type_name::<T>()));
        }
        
        lock.insert(type_id, Box::new(instance));
        Ok(())
    }

    /// Resolves a registered service from the container.
    /// Requires `T` to implement `Clone`. Since services are typically registered as `Arc<T>`,
    /// cloning merely increments the reference count, preserving the singleton lifetime.
    pub async fn resolve<T: Any + Send + Sync + Clone>(&self) -> Result<T, ContainerError> {
        let type_id = TypeId::of::<T>();
        let lock = self.services.read().await;
        
        if let Some(boxed_any) = lock.get(&type_id) {
            if let Some(instance) = boxed_any.downcast_ref::<T>() {
                return Ok(instance.clone());
            }
        }
        
        Err(ContainerError::NotFound(std::any::type_name::<T>()))
    }

    /// Checks if a service of the specified type is currently registered.
    pub async fn contains<T: Any + Send + Sync>(&self) -> bool {
        let type_id = TypeId::of::<T>();
        let lock = self.services.read().await;
        lock.contains_key(&type_id)
    }

    /// Removes a service from the container.
    pub async fn unregister<T: Any + Send + Sync>(&self) -> Result<(), ContainerError> {
        let type_id = TypeId::of::<T>();
        let mut lock = self.services.write().await;
        
        if lock.remove(&type_id).is_none() {
            return Err(ContainerError::NotFound(std::any::type_name::<T>()));
        }
        
        Ok(())
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
    use std::sync::Arc;

    // Define a dummy trait to test interface-based registration
    trait TestService: Send + Sync {
        fn get_value(&self) -> i32;
    }

    #[derive(Debug)]
    struct ConcreteService {
        value: i32,
    }

    impl TestService for ConcreteService {
        fn get_value(&self) -> i32 {
            self.value
        }
    }

    #[tokio::test]
    async fn test_register_and_resolve_concrete() {
        let container = ServiceContainer::new();
        let instance = Arc::new(ConcreteService { value: 42 });
        
        container.register(instance.clone()).await.unwrap();
        
        let resolved = container.resolve::<Arc<ConcreteService>>().await.unwrap();
        assert_eq!(resolved.get_value(), 42);
    }

    #[tokio::test]
    async fn test_register_and_resolve_trait() {
        let container = ServiceContainer::new();
        // Type explicitly forced to the Trait object
        let instance: Arc<dyn TestService> = Arc::new(ConcreteService { value: 100 });
        
        container.register(instance.clone()).await.unwrap();
        
        // Resolve by Trait object
        let resolved = container.resolve::<Arc<dyn TestService>>().await.unwrap();
        assert_eq!(resolved.get_value(), 100);
    }

    #[tokio::test]
    async fn test_duplicate_registration_fails() {
        let container = ServiceContainer::new();
        let instance = Arc::new(ConcreteService { value: 1 });
        
        container.register(instance.clone()).await.unwrap();
        let err = container.register(instance.clone()).await.unwrap_err();
        
        assert!(matches!(err, ContainerError::AlreadyRegistered(_)));
    }

    #[tokio::test]
    async fn test_resolve_not_found() {
        let container = ServiceContainer::new();
        let err = container.resolve::<Arc<ConcreteService>>().await.unwrap_err();
        
        assert!(matches!(err, ContainerError::NotFound(_)));
    }

    #[tokio::test]
    async fn test_contains_and_unregister() {
        let container = ServiceContainer::new();
        let instance = Arc::new(ConcreteService { value: 1 });
        
        assert!(!container.contains::<Arc<ConcreteService>>().await);
        
        container.register(instance).await.unwrap();
        assert!(container.contains::<Arc<ConcreteService>>().await);
        
        container.unregister::<Arc<ConcreteService>>().await.unwrap();
        assert!(!container.contains::<Arc<ConcreteService>>().await);
        
        let err = container.unregister::<Arc<ConcreteService>>().await.unwrap_err();
        assert!(matches!(err, ContainerError::NotFound(_)));
    }
}
