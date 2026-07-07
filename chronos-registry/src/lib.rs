//! # Chronos Service Registry
//!
//! The Service Registry is the canonical runtime catalog of every active subsystem in the PCOS.
//! It is responsible only for service discovery, lifecycle tracking, capability advertisement,
//! and health reporting. 
//!
//! It is NOT responsible for dependency injection, configuration, business logic, or execution.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Errors that can occur within the ServiceRegistry.
#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    #[error("Service ID {0} is already registered")]
    AlreadyRegistered(String),
    #[error("Service ID {0} not found")]
    NotFound(String),
}

/// The operational status of a service.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServiceStatus {
    Starting,
    Running,
    Paused,
    Stopping,
    Stopped,
    Failed,
}

/// The health state of a service.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServiceHealth {
    Healthy,
    Degraded,
    Unavailable,
    Unknown,
}

/// The architectural layer or type of a service.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServiceType {
    Adapter,
    Engine,
    Storage,
    Transport,
    Connector,
    UI,
    Plugin,
    Core,
}

/// Immutable metadata describing a registered service.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceDescriptor {
    pub service_id: String,
    pub service_name: String,
    pub service_type: ServiceType,
    pub semantic_version: String,
    pub startup_timestamp: DateTime<Utc>,
    pub current_status: ServiceStatus,
    pub health_state: ServiceHealth,
    pub registered_capabilities: Vec<String>,
    pub consumed_runtime_objects: Vec<String>,
    pub produced_runtime_objects: Vec<String>,
}

impl ServiceDescriptor {
    /// Creates a new `ServiceDescriptor` with default initial states (Starting, Unknown).
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        service_id: impl Into<String>,
        service_name: impl Into<String>,
        service_type: ServiceType,
        semantic_version: impl Into<String>,
        registered_capabilities: Vec<String>,
        consumed_runtime_objects: Vec<String>,
        produced_runtime_objects: Vec<String>,
    ) -> Self {
        Self {
            service_id: service_id.into(),
            service_name: service_name.into(),
            service_type,
            semantic_version: semantic_version.into(),
            startup_timestamp: Utc::now(),
            current_status: ServiceStatus::Starting,
            health_state: ServiceHealth::Unknown,
            registered_capabilities,
            consumed_runtime_objects,
            produced_runtime_objects,
        }
    }
}

/// The runtime catalog of every active subsystem in the PCOS.
#[derive(Clone)]
pub struct ServiceRegistry {
    services: Arc<RwLock<HashMap<String, ServiceDescriptor>>>,
}

impl ServiceRegistry {
    /// Creates a new, empty ServiceRegistry.
    pub fn new() -> Self {
        Self {
            services: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Registers a new service descriptor. Fails if the service ID already exists.
    pub async fn register(&self, descriptor: ServiceDescriptor) -> Result<(), RegistryError> {
        let mut lock = self.services.write().await;
        if lock.contains_key(&descriptor.service_id) {
            return Err(RegistryError::AlreadyRegistered(descriptor.service_id));
        }
        lock.insert(descriptor.service_id.clone(), descriptor);
        Ok(())
    }

    /// Unregisters a service by its ID. Fails if the service does not exist.
    pub async fn unregister(&self, service_id: &str) -> Result<(), RegistryError> {
        let mut lock = self.services.write().await;
        if lock.remove(service_id).is_none() {
            return Err(RegistryError::NotFound(service_id.to_string()));
        }
        Ok(())
    }

    /// Updates the status of an existing service. Fails if the service does not exist.
    pub async fn update_status(&self, service_id: &str, status: ServiceStatus) -> Result<(), RegistryError> {
        let mut lock = self.services.write().await;
        if let Some(descriptor) = lock.get_mut(service_id) {
            descriptor.current_status = status;
            Ok(())
        } else {
            Err(RegistryError::NotFound(service_id.to_string()))
        }
    }

    /// Updates the health state of an existing service. Fails if the service does not exist.
    pub async fn update_health(&self, service_id: &str, health: ServiceHealth) -> Result<(), RegistryError> {
        let mut lock = self.services.write().await;
        if let Some(descriptor) = lock.get_mut(service_id) {
            descriptor.health_state = health;
            Ok(())
        } else {
            Err(RegistryError::NotFound(service_id.to_string()))
        }
    }

    /// Looks up a service descriptor by its ID.
    pub async fn lookup(&self, service_id: &str) -> Option<ServiceDescriptor> {
        let lock = self.services.read().await;
        lock.get(service_id).cloned()
    }

    /// Lists all registered service descriptors.
    pub async fn list(&self) -> Vec<ServiceDescriptor> {
        let lock = self.services.read().await;
        lock.values().cloned().collect()
    }

    /// Lists all registered service descriptors matching the given type.
    pub async fn list_by_type(&self, service_type: &ServiceType) -> Vec<ServiceDescriptor> {
        let lock = self.services.read().await;
        lock.values()
            .filter(|d| &d.service_type == service_type)
            .cloned()
            .collect()
    }

    /// Lists all registered service descriptors declaring the given capability.
    pub async fn list_by_capability(&self, capability: &str) -> Vec<ServiceDescriptor> {
        let lock = self.services.read().await;
        lock.values()
            .filter(|d| d.registered_capabilities.iter().any(|c| c == capability))
            .cloned()
            .collect()
    }

    /// Checks if a service ID exists in the registry.
    pub async fn exists(&self, service_id: &str) -> bool {
        let lock = self.services.read().await;
        lock.contains_key(service_id)
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

    fn create_test_descriptor(id: &str, cap: &str, s_type: ServiceType) -> ServiceDescriptor {
        ServiceDescriptor::new(
            id,
            "Test Service",
            s_type,
            "1.0.0",
            vec![cap.to_string()],
            vec!["ChronosState".to_string()],
            vec!["ChronosDecision".to_string()],
        )
    }

    #[tokio::test]
    async fn test_register_and_lookup() {
        let registry = ServiceRegistry::new();
        let desc = create_test_descriptor("svc_1", "ObserveGit", ServiceType::Adapter);
        
        registry.register(desc.clone()).await.unwrap();
        
        let found = registry.lookup("svc_1").await.unwrap();
        assert_eq!(found.service_name, "Test Service");
        
        // Ensure duplicate registration fails
        let err = registry.register(desc).await.unwrap_err();
        assert!(matches!(err, RegistryError::AlreadyRegistered(_)));
    }

    #[tokio::test]
    async fn test_unregister() {
        let registry = ServiceRegistry::new();
        let desc = create_test_descriptor("svc_1", "Cap", ServiceType::Adapter);
        
        registry.register(desc).await.unwrap();
        assert!(registry.exists("svc_1").await);
        
        registry.unregister("svc_1").await.unwrap();
        assert!(!registry.exists("svc_1").await);
        
        // Ensure unregistering non-existent service fails
        let err = registry.unregister("svc_1").await.unwrap_err();
        assert!(matches!(err, RegistryError::NotFound(_)));
    }

    #[tokio::test]
    async fn test_update_status_and_health() {
        let registry = ServiceRegistry::new();
        let desc = create_test_descriptor("svc_1", "Cap", ServiceType::Adapter);
        
        registry.register(desc).await.unwrap();
        
        registry.update_status("svc_1", ServiceStatus::Running).await.unwrap();
        registry.update_health("svc_1", ServiceHealth::Healthy).await.unwrap();
        
        let found = registry.lookup("svc_1").await.unwrap();
        assert_eq!(found.current_status, ServiceStatus::Running);
        assert_eq!(found.health_state, ServiceHealth::Healthy);
    }

    #[tokio::test]
    async fn test_list_filters() {
        let registry = ServiceRegistry::new();
        let desc1 = create_test_descriptor("svc_1", "ObserveGit", ServiceType::Adapter);
        let desc2 = create_test_descriptor("svc_2", "Reasoning", ServiceType::Engine);
        let desc3 = create_test_descriptor("svc_3", "ObserveGit", ServiceType::Adapter);
        
        registry.register(desc1).await.unwrap();
        registry.register(desc2).await.unwrap();
        registry.register(desc3).await.unwrap();
        
        assert_eq!(registry.list().await.len(), 3);
        
        let adapters = registry.list_by_type(&ServiceType::Adapter).await;
        assert_eq!(adapters.len(), 2);
        
        let engines = registry.list_by_type(&ServiceType::Engine).await;
        assert_eq!(engines.len(), 1);
        
        let git_observers = registry.list_by_capability("ObserveGit").await;
        assert_eq!(git_observers.len(), 2);
        
        let missing = registry.list_by_capability("MissingCap").await;
        assert_eq!(missing.len(), 0);
    }
}
