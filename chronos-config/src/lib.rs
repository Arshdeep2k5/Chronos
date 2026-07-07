//! # Chronos Configuration Service
//!
//! The canonical configuration service for the Chronos Cognitive Kernel.
//! All subsystems must retrieve configuration through this service.
//! No subsystem may read configuration files directly.
//!
//! # Operational Profile
//! * **Consumes:** Config files, environment defaults, memory maps.
//! * **Produces:** Type-safe configuration structs for dependent services.
//! * **Capabilities:** Hierarchical key resolution, fallback chains, type-safe deserialization.
//! * **Dependencies:** `serde`, `serde_json`.
//! * **Failure Modes:** Missing keys, parse errors, type mismatch.

use async_trait::async_trait;
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Errors that can occur within the Configuration Service.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Configuration key not found: {0}")]
    NotFound(String),
    #[error("Failed to parse or deserialize configuration: {0}")]
    ParseError(String),
    #[error("Configuration validation failed: {0}")]
    ValidationError(String),
    #[error("Provider error: {0}")]
    ProviderError(String),
}

/// The base trait for any configuration source.
#[async_trait]
pub trait ConfigurationProvider: Send + Sync {
    /// Attempts to retrieve a JSON value for a specific dot-separated key (e.g., `db.port`).
    async fn get_value(&self, key: &str) -> Result<Option<Value>, ConfigError>;
}

/// An in-memory configuration provider, typically used for overrides or testing.
pub struct MemoryConfigurationProvider {
    config: Arc<RwLock<Value>>,
}

impl MemoryConfigurationProvider {
    /// Creates a new MemoryConfigurationProvider from an initial JSON object.
    pub fn new(initial_config: Value) -> Self {
        Self {
            config: Arc::new(RwLock::new(initial_config)),
        }
    }

    /// Sets a value for a specific top-level key.
    pub async fn set(&self, key: &str, value: Value) {
        let mut lock = self.config.write().await;
        if let Some(obj) = lock.as_object_mut() {
            obj.insert(key.to_string(), value);
        }
    }
}

#[async_trait]
impl ConfigurationProvider for MemoryConfigurationProvider {
    async fn get_value(&self, key: &str) -> Result<Option<Value>, ConfigError> {
        let lock = self.config.read().await;
        resolve_json_pointer(&lock, key)
    }
}

/// A file-backed configuration provider reading from a JSON file.
pub struct FileConfigurationProvider {
    file_path: PathBuf,
    cache: Arc<RwLock<Option<Value>>>,
}

impl FileConfigurationProvider {
    /// Creates a new FileConfigurationProvider.
    pub fn new(file_path: PathBuf) -> Self {
        Self {
            file_path,
            cache: Arc::new(RwLock::new(None)),
        }
    }

    /// Loads or reloads the JSON file into the cache.
    pub async fn load(&self) -> Result<(), ConfigError> {
        let content = tokio::fs::read_to_string(&self.file_path)
            .await
            .map_err(|e| ConfigError::ProviderError(format!("Failed to read file: {}", e)))?;
        
        let json: Value = serde_json::from_str(&content)
            .map_err(|e| ConfigError::ParseError(format!("Invalid JSON: {}", e)))?;
            
        let mut lock = self.cache.write().await;
        *lock = Some(json);
        Ok(())
    }
}

#[async_trait]
impl ConfigurationProvider for FileConfigurationProvider {
    async fn get_value(&self, key: &str) -> Result<Option<Value>, ConfigError> {
        let lock = self.cache.read().await;
        if let Some(json) = lock.as_ref() {
            resolve_json_pointer(json, key)
        } else {
            Ok(None)
        }
    }
}

/// Helper function to resolve dot-notation keys (e.g., "server.port") against a JSON value.
fn resolve_json_pointer(json: &Value, key: &str) -> Result<Option<Value>, ConfigError> {
    let mut current = json;
    for part in key.split('.') {
        match current {
            Value::Object(map) => {
                if let Some(val) = map.get(part) {
                    current = val;
                } else {
                    return Ok(None);
                }
            }
            _ => return Ok(None),
        }
    }
    Ok(Some(current.clone()))
}

/// The main Configuration Service that aggregates multiple providers.
/// Providers registered earlier have higher priority.
#[derive(Clone)]
pub struct ConfigurationService {
    providers: Arc<RwLock<Vec<Arc<dyn ConfigurationProvider>>>>,
}

impl ConfigurationService {
    /// Creates a new, empty ConfigurationService.
    pub fn new() -> Self {
        Self {
            providers: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Adds a configuration provider to the service. 
    /// Providers are queried in the order they are added.
    pub async fn add_provider(&self, provider: Arc<dyn ConfigurationProvider>) {
        let mut lock = self.providers.write().await;
        lock.push(provider);
    }

    /// Retrieves a type-safe configuration value by its hierarchical key.
    pub async fn get<T: DeserializeOwned>(&self, key: &str) -> Result<T, ConfigError> {
        let lock = self.providers.read().await;
        
        for provider in lock.iter() {
            if let Some(value) = provider.get_value(key).await? {
                return serde_json::from_value(value).map_err(|e| {
                    ConfigError::ParseError(format!("Type mismatch for key '{}': {}", key, e))
                });
            }
        }
        
        Err(ConfigError::NotFound(key.to_string()))
    }

    /// Retrieves a configuration value, returning a default if not found.
    pub async fn get_or_default<T: DeserializeOwned>(&self, key: &str, default: T) -> Result<T, ConfigError> {
        match self.get::<T>(key).await {
            Ok(val) => Ok(val),
            Err(ConfigError::NotFound(_)) => Ok(default),
            Err(e) => Err(e),
        }
    }

    /// Retrieves a configuration value and runs a custom validation closure.
    pub async fn get_validated<T, F>(&self, key: &str, validator: F) -> Result<T, ConfigError>
    where
        T: DeserializeOwned,
        F: Fn(&T) -> Result<(), String>,
    {
        let val = self.get::<T>(key).await?;
        if let Err(e) = validator(&val) {
            return Err(ConfigError::ValidationError(e));
        }
        Ok(val)
    }
}

impl Default for ConfigurationService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_memory_provider_and_hierarchical_keys() {
        let initial = json!({
            "database": {
                "host": "localhost",
                "port": 5432
            }
        });
        
        let mem_provider = Arc::new(MemoryConfigurationProvider::new(initial));
        let service = ConfigurationService::new();
        service.add_provider(mem_provider).await;

        let host: String = service.get("database.host").await.unwrap();
        assert_eq!(host, "localhost");

        let port: u16 = service.get("database.port").await.unwrap();
        assert_eq!(port, 5432);
        
        let missing = service.get::<String>("database.missing").await;
        assert!(matches!(missing, Err(ConfigError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_get_or_default() {
        let mem_provider = Arc::new(MemoryConfigurationProvider::new(json!({})));
        let service = ConfigurationService::new();
        service.add_provider(mem_provider).await;

        let port: u16 = service.get_or_default("server.port", 8080).await.unwrap();
        assert_eq!(port, 8080);
    }

    #[tokio::test]
    async fn test_validation() {
        let mem_provider = Arc::new(MemoryConfigurationProvider::new(json!({ "age": -5 })));
        let service = ConfigurationService::new();
        service.add_provider(mem_provider).await;

        let result = service.get_validated::<i32, _>("age", |&v| {
            if v < 0 { Err("Age cannot be negative".to_string()) } else { Ok(()) }
        }).await;

        assert!(matches!(result, Err(ConfigError::ValidationError(_))));
    }

    #[tokio::test]
    async fn test_file_provider() {
        let mut temp_file = NamedTempFile::new().unwrap();
        write!(temp_file, r#"{{"api": {{"url": "https://chronos.local"}}}}"#).unwrap();
        
        let file_provider = Arc::new(FileConfigurationProvider::new(temp_file.path().to_path_buf()));
        file_provider.load().await.unwrap();
        
        let service = ConfigurationService::new();
        service.add_provider(file_provider).await;
        
        let url: String = service.get("api.url").await.unwrap();
        assert_eq!(url, "https://chronos.local");
    }

    #[tokio::test]
    async fn test_provider_chaining() {
        // Provider 1 (Primary) has "port": 9090
        let mem1 = Arc::new(MemoryConfigurationProvider::new(json!({ "port": 9090 })));
        
        // Provider 2 (Fallback) has "port": 8080 and "host": "127.0.0.1"
        let mem2 = Arc::new(MemoryConfigurationProvider::new(json!({ "port": 8080, "host": "127.0.0.1" })));
        
        let service = ConfigurationService::new();
        service.add_provider(mem1).await;
        service.add_provider(mem2).await;
        
        // Should resolve from Primary
        let port: u16 = service.get("port").await.unwrap();
        assert_eq!(port, 9090);
        
        // Should fallback to Secondary
        let host: String = service.get("host").await.unwrap();
        assert_eq!(host, "127.0.0.1");
    }
}
