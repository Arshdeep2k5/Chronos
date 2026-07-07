//! # Chronos Filesystem Watcher Perception Adapter
//!
//! Watches configured directory structures recursively for file mutations, normalizes
//! events, and publishes them onto the Cognitive Bus.


use chrono::Utc;
use chronos_bus::EventBus;
use chronos_config::ConfigurationService;
use chronos_core::ChronosEvent;
use chronos_logging::{ChronosLogger, LogContext};
use chronos_registry::{ServiceDescriptor, ServiceRegistry, ServiceType};
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde_json::json;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

/// Errors that can occur within the Filesystem Watcher.
#[derive(Debug, thiserror::Error)]
pub enum WatcherError {
    #[error("Registry error: {0}")]
    Registry(String),
    #[error("Bus error: {0}")]
    Bus(String),
    #[error("Notify error: {0}")]
    Notify(notify::Error),
    #[error("IO error: {0}")]
    Io(String),
}

/// Normalizes OS filesystem notification events into schema-compliant `ChronosEvent`s.
pub struct FilewatcherNormalizer;

impl FilewatcherNormalizer {
    /// Maps a raw notify::Event into a ChronosEvent if applicable.
    pub fn normalize(event: &Event) -> Option<ChronosEvent> {
        let timestamp = Utc::now();
        let path = event.paths.first()?;
        let extension = path.extension()
            .map(|ext| ext.to_string_lossy().to_string())
            .unwrap_or_default();
            
        let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);

        let event_type = match event.kind {
            EventKind::Create(_) => "FileCreated",
            EventKind::Modify(notify::event::ModifyKind::Name(_)) => "FileMoved",
            EventKind::Modify(_) => "FileModified",
            EventKind::Remove(_) => "FileDeleted",
            _ => return None,
        };

        let mut payload = json!({
            "path": path.to_string_lossy(),
            "extension": extension,
            "size": size,
            "timestamp": timestamp.to_rfc3339(),
            "repository_id": null,
        });

        // Add additional move details if it's a rename event containing two paths
        if event_type == "FileMoved" && event.paths.len() >= 2 {
            let old_path = &event.paths[0];
            let new_path = &event.paths[1];
            payload["old_path"] = json!(old_path.to_string_lossy());
            payload["new_path"] = json!(new_path.to_string_lossy());
        }

        Some(ChronosEvent::new(event_type, "FilewatcherAdapter", payload))
    }
}

/// The main Filesystem Watcher Adapter implementing the async observer loop.
pub struct FilewatcherAdapter {
    registry: Arc<ServiceRegistry>,
    bus: Arc<dyn EventBus>,
    _config: Arc<ConfigurationService>,
    logger: ChronosLogger,
    watcher: Arc<Mutex<Option<RecommendedWatcher>>>,
    debounce_map: Arc<Mutex<HashMap<PathBuf, Instant>>>,
}

impl FilewatcherAdapter {
    pub fn new(
        registry: Arc<ServiceRegistry>,
        bus: Arc<dyn EventBus>,
        config: Arc<ConfigurationService>,
        logger: ChronosLogger,
    ) -> Self {
        Self {
            registry,
            bus,
            _config: config,
            logger,
            watcher: Arc::new(Mutex::new(None)),
            debounce_map: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Starts the file watcher adapter, registering capabilities.
    pub async fn start(&self) -> Result<(), WatcherError> {
        let desc = ServiceDescriptor::new(
            "chronos-adapter-filewatcher",
            "Filesystem Watcher Adapter",
            ServiceType::Adapter,
            "1.0.0",
            vec!["ObserveFilesystem".to_string()],
            vec![],
            vec![
                "FileCreated".to_string(),
                "FileModified".to_string(),
                "FileDeleted".to_string(),
                "FileMoved".to_string(),
            ],
        );

        self.registry.register(desc)
            .await
            .map_err(|e| WatcherError::Registry(e.to_string()))?;

        self.logger.info("Filesystem Watcher Adapter started and registered.", None);
        Ok(())
    }

    /// Sets up a recursive directory watch on the specified path with debouncing.
    pub fn watch_directory(&self, path: PathBuf) -> Result<(), WatcherError> {
        let bus = self.bus.clone();
        let logger = self.logger.clone();
        let debounce_map = self.debounce_map.clone();
        
        let (tx, mut rx) = mpsc::channel::<Event>(100);

        // Async event processor loop to handle debouncing and event publication
        tokio::spawn(async move {
            while let Some(raw_event) = rx.recv().await {
                // Apply debounce rules (500ms modify windows)
                if let EventKind::Modify(_) = raw_event.kind {
                    if let Some(first_path) = raw_event.paths.first() {
                        let now = Instant::now();
                        let mut map = debounce_map.lock().unwrap();
                        
                        if let Some(last_time) = map.get(first_path) {
                            if now.duration_since(*last_time) < Duration::from_millis(500) {
                                // Skip modify events within the debounce window
                                continue;
                            }
                        }
                        map.insert(first_path.clone(), now);
                    }
                }

                if let Some(chronos_event) = FilewatcherNormalizer::normalize(&raw_event) {
                    logger.info(
                        &format!("Filesystem event: {}", chronos_event.event_type),
                        Some(LogContext::new().with_field("file_event_id", &chronos_event.id)),
                    );
                    let _ = bus.publish(chronos_event);
                }
            }
        });

        // Initialize standard watcher
        let mut watcher = RecommendedWatcher::new(move |res| {
            match res {
                Ok(event) => {
                    let _ = tx.blocking_send(event);
                }
                Err(e) => {
                    // Log notify error
                    eprintln!("Notify error occurred: {}", e);
                }
            }
        }, Config::default()).map_err(WatcherError::Notify)?;

        watcher.watch(&path, RecursiveMode::Recursive)
            .map_err(WatcherError::Notify)?;

        let mut lock = self.watcher.lock().unwrap();
        *lock = Some(watcher);

        self.logger.info(&format!("Watching directory recursively: {:?}", path), None);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chronos_bus::MemoryEventBus;
    use std::io::Write;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_filewatcher_adapter_lifecycle() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");

        let registry = Arc::new(ServiceRegistry::new());
        let bus = Arc::new(MemoryEventBus::new(100));
        let config = Arc::new(ConfigurationService::new());
        let logger = ChronosLogger::new(LogContext::new());

        let adapter = FilewatcherAdapter::new(registry, bus.clone(), config, logger);
        let mut sub = bus.subscribe();

        adapter.start().await.unwrap();
        adapter.watch_directory(dir.path().to_path_buf()).unwrap();

        // Write a file to trigger notify event
        {
            let mut file = std::fs::File::create(&file_path).unwrap();
            file.write_all(b"Hello world").unwrap();
        }

        // We should receive FileCreated and/or FileModified
        let mut found_create = false;
        let mut found_modify = false;

        // Give file watcher some time to yield OS event loop
        for _ in 0..10 {
            tokio::time::sleep(Duration::from_millis(50)).await;
            while let Ok(evt) = sub.next_event().await {
                if evt.event_type == "FileCreated" {
                    found_create = true;
                }
                if evt.event_type == "FileModified" {
                    found_modify = true;
                }
                if found_create || found_modify {
                    break;
                }
            }
            if found_create || found_modify {
                break;
            }
        }

        assert!(found_create || found_modify);
    }
}
