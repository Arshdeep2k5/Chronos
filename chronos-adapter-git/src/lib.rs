//! # Chronos Git Perception Adapter
//!
//! Observes Git repositories, normalizes commit/checkout activity into ChronosEvents,
//! and publishes them onto the Cognitive Bus.

use chrono::{DateTime, Utc};
use chronos_bus::EventBus;
use chronos_config::ConfigurationService;
use chronos_core::ChronosEvent;
use chronos_logging::{ChronosLogger, LogContext};
use chronos_registry::{ServiceDescriptor, ServiceRegistry, ServiceType};
use serde_json::json;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use tokio::sync::RwLock;

/// Errors that can occur within the Git Adapter.
#[derive(Debug, thiserror::Error)]
pub enum AdapterError {
    #[error("Registry error: {0}")]
    Registry(String),
    #[error("Bus error: {0}")]
    Bus(String),
    #[error("Configuration error: {0}")]
    Config(String),
    #[error("IO error: {0}")]
    Io(String),
}

/// Normalizes raw Git reflog logs into schema-compliant `ChronosEvent` models.
pub struct GitEventNormalizer;

impl GitEventNormalizer {
    /// Normalizes a single reflog entry line.
    pub fn normalize(
        repo_path: &Path,
        repo_name: &str,
        branch: &str,
        line: &str,
    ) -> Option<ChronosEvent> {
        // Line format: <old-sha> <new-sha> <committer> <email> <timestamp> <tz> <message>
        // Example: 000000 1a2b3c User <email> 1672531199 +0000 commit: Implement feature
        let parts: Vec<&str> = line.splitn(7, ' ').collect();
        if parts.len() < 7 {
            return None;
        }

        let old_sha = parts[0];
        let new_sha = parts[1];
        let committer_info = parts[2];
        let committer_email = parts[3];
        let raw_timestamp = parts[4];
        let git_message = parts[6];

        // Determine event type based on message prefixes
        let (event_type, payload) = if git_message.starts_with("commit:") {
            ("GitCommitCreated", json!({
                "sha": new_sha,
                "old_sha": old_sha,
                "message": git_message.trim_start_matches("commit:").trim(),
                "committer": committer_info,
                "committer_email": committer_email,
            }))
        } else if git_message.starts_with("commit (amend):") {
            ("GitCommitAmended", json!({
                "sha": new_sha,
                "old_sha": old_sha,
                "message": git_message.trim_start_matches("commit (amend):").trim(),
                "committer": committer_info,
                "committer_email": committer_email,
            }))
        } else if git_message.starts_with("checkout:") {
            // "checkout: moving from master to dev"
            let msg = git_message.trim_start_matches("checkout:").trim();
            let checkout_parts: Vec<&str> = msg.split(" to ").collect();
            let from_branch = checkout_parts.first().map(|s| s.trim_start_matches("moving from").trim()).unwrap_or("");
            let to_branch = checkout_parts.get(1).map(|s| s.trim()).unwrap_or("");
            
            ("GitBranchSwitched", json!({
                "from_branch": from_branch,
                "to_branch": to_branch,
                "sha": new_sha,
            }))
        } else if git_message.starts_with("merge") {
            ("GitMergePerformed", json!({
                "sha": new_sha,
                "old_sha": old_sha,
                "message": git_message.trim(),
                "committer": committer_info,
                "committer_email": committer_email,
            }))
        } else if git_message.starts_with("tag:") {
            ("GitTagCreated", json!({
                "sha": new_sha,
                "tag_name": git_message.trim_start_matches("tag:").trim(),
            }))
        } else {
            ("GitRepositoryModified", json!({
                "sha": new_sha,
                "old_sha": old_sha,
                "message": git_message.trim(),
            }))
        };

        // Parse Git time
        let ts_sec = raw_timestamp.parse::<i64>().unwrap_or_else(|_| Utc::now().timestamp());
        let event_time = DateTime::from_timestamp(ts_sec, 0).unwrap_or_else(|| Utc::now());

        let final_payload = json!({
            "repository_path": repo_path.to_string_lossy(),
            "repository_name": repo_name,
            "branch": branch,
            "adapter_id": "chronos-adapter-git",
            "git_event_type": event_type,
            "source_payload": payload,
        });

        let mut event = ChronosEvent::new(event_type, "GitAdapter", final_payload);
        event.timestamp = event_time;
        Some(event)
    }
}

/// Helper publisher implementing context wrapper.
pub struct GitEventPublisher {
    bus: Arc<dyn EventBus>,
    logger: ChronosLogger,
}

impl GitEventPublisher {
    pub fn new(bus: Arc<dyn EventBus>, logger: ChronosLogger) -> Self {
        Self { bus, logger }
    }

    pub fn publish(&self, event: ChronosEvent) -> Result<(), AdapterError> {
        self.logger.info(
            &format!("Publishing Git event: {}", event.event_type),
            Some(LogContext::new().with_field("git_event_id", &event.id)),
        );
        self.bus.publish(event).map_err(|e| AdapterError::Bus(e.to_string()))?;
        Ok(())
    }
}

/// Monitors a single Git repository directory for state mutations.
pub struct GitRepositoryObserver {
    repo_path: PathBuf,
    repo_name: String,
    last_processed_line: usize,
}

impl GitRepositoryObserver {
    pub fn new(repo_path: PathBuf) -> Self {
        let repo_name = repo_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown-repo".to_string());
            
        Self {
            repo_path,
            repo_name,
            last_processed_line: 0,
        }
    }

    /// Reads the current branch name from `.git/HEAD`.
    pub async fn current_branch(&self) -> String {
        let head_path = self.repo_path.join(".git").join("HEAD");
        if let Ok(content) = fs::read_to_string(head_path).await {
            let line = content.trim();
            if line.starts_with("ref: refs/heads/") {
                return line.trim_start_matches("ref: refs/heads/").to_string();
            }
            return "DETACHED".to_string();
        }
        "UNKNOWN".to_string()
    }

    /// Checks the `.git/logs/HEAD` file for new reflog events.
    pub async fn poll_new_events(&mut self) -> Result<Vec<ChronosEvent>, AdapterError> {
        let log_path = self.repo_path.join(".git").join("logs").join("HEAD");
        if !log_path.exists() {
            return Ok(Vec::new());
        }

        let content = fs::read_to_string(&log_path)
            .await
            .map_err(|e| AdapterError::Io(e.to_string()))?;
            
        let lines: Vec<&str> = content.lines().collect();
        let total_lines = lines.len();
        
        let mut new_events = Vec::new();
        let branch = self.current_branch().await;

        if self.last_processed_line < total_lines {
            for idx in self.last_processed_line..total_lines {
                let line = lines[idx];
                if let Some(event) = GitEventNormalizer::normalize(&self.repo_path, &self.repo_name, &branch, line) {
                    new_events.push(event);
                }
            }
            self.last_processed_line = total_lines;
        }

        Ok(new_events)
    }

    /// Discovers if path contains a valid git repo structure.
    pub fn is_valid_git_repo(path: &Path) -> bool {
        path.join(".git").exists()
    }
}

/// The main Git Adapter managing registration and active background polling.
pub struct GitAdapter {
    registry: Arc<ServiceRegistry>,
    bus: Arc<dyn EventBus>,
    _config: Arc<ConfigurationService>,
    logger: ChronosLogger,
    observers: Arc<RwLock<HashMap<PathBuf, GitRepositoryObserver>>>,
}

impl GitAdapter {
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
            observers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Starts the adapter lifecycle, registering with the system registry.
    pub async fn start(&self) -> Result<(), AdapterError> {
        let desc = ServiceDescriptor::new(
            "chronos-adapter-git",
            "Git Repository Observer",
            ServiceType::Adapter,
            "1.0.0",
            vec!["ObserveGit".to_string()],
            vec![],
            vec![
                "GitCommitCreated".to_string(),
                "GitBranchSwitched".to_string(),
                "GitMergePerformed".to_string(),
                "GitTagCreated".to_string(),
            ],
        );

        self.registry.register(desc)
            .await
            .map_err(|e| AdapterError::Registry(e.to_string()))?;
            
        self.logger.info("Git Adapter started and registered.", None);
        Ok(())
    }

    /// Manually registers a repository path to monitor.
    pub async fn watch_repository(&self, path: PathBuf) -> Result<(), AdapterError> {
        if !GitRepositoryObserver::is_valid_git_repo(&path) {
            return Err(AdapterError::Io(format!("Not a valid git repository: {:?}", path)));
        }

        let mut observer = GitRepositoryObserver::new(path.clone());
        
        // Warm the cache by mapping existing reflog entries to the last processed line
        let log_path = path.join(".git").join("logs").join("HEAD");
        if log_path.exists() {
            if let Ok(content) = fs::read_to_string(&log_path).await {
                observer.last_processed_line = content.lines().count();
            }
        }

        let mut lock = self.observers.write().await;
        lock.insert(path.clone(), observer);
        
        // Publish Discovery Event
        let event = ChronosEvent::new(
            "GitRepositoryDiscovered",
            "GitAdapter",
            json!({
                "repository_path": path.to_string_lossy(),
                "adapter_id": "chronos-adapter-git"
            }),
        );
        self.bus.publish(event).map_err(|e| AdapterError::Bus(e.to_string()))?;

        self.logger.info(&format!("Watching Git repository at: {:?}", path), None);
        Ok(())
    }

    /// Performs a single pass check across all watched repositories.
    pub async fn poll(&self) -> Result<(), AdapterError> {
        let mut lock = self.observers.write().await;
        let publisher = GitEventPublisher::new(self.bus.clone(), self.logger.clone());

        for observer in lock.values_mut() {
            let events = observer.poll_new_events().await?;
            for event in events {
                publisher.publish(event)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chronos_bus::MemoryEventBus;
    use std::io::Write;
    use tempfile::tempdir;

    fn setup_mock_reflog(git_dir: &Path, content: &str) {
        let logs_dir = git_dir.join("logs");
        std::fs::create_dir_all(&logs_dir).unwrap();
        let mut file = std::fs::File::create(logs_dir.join("HEAD")).unwrap();
        file.write_all(content.as_bytes()).unwrap();
    }

    fn setup_mock_head(git_dir: &Path, content: &str) {
        let mut file = std::fs::File::create(git_dir.join("HEAD")).unwrap();
        file.write_all(content.as_bytes()).unwrap();
    }

    #[tokio::test]
    async fn test_git_discovery_and_observer_lifecycle() {
        let dir = tempdir().unwrap();
        let repo_path = dir.path().to_path_buf();
        let git_dir = repo_path.join(".git");
        std::fs::create_dir_all(&git_dir).unwrap();

        assert!(GitRepositoryObserver::is_valid_git_repo(&repo_path));

        // Create initial config, registry, logger, and bus
        let registry = Arc::new(ServiceRegistry::new());
        let bus = Arc::new(MemoryEventBus::new(100));
        let config = Arc::new(ConfigurationService::new());
        
        let logger = ChronosLogger::new(LogContext::new());
        let adapter = GitAdapter::new(registry.clone(), bus.clone(), config, logger);
        
        let mut sub = bus.subscribe();

        // Start adapter and register repo
        adapter.start().await.unwrap();
        adapter.watch_repository(repo_path.clone()).await.unwrap();

        // Should receive the discovery event
        let disc_evt = sub.next_event().await.unwrap();
        assert_eq!(disc_evt.event_type, "GitRepositoryDiscovered");
        assert_eq!(disc_evt.payload["repository_path"].as_str().unwrap(), repo_path.to_string_lossy());
    }

    #[test]
    fn test_git_event_normalization() {
        let path = Path::new("/workspace/test-repo");
        
        // Commit Event log line
        let commit_line = "0000000000000000000000000000000000000000 1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b User <email@example.com> 1672531199 +0000 commit: Implement kernel config module";
        let event = GitEventNormalizer::normalize(path, "test-repo", "main", commit_line).unwrap();
        
        assert_eq!(event.event_type, "GitCommitCreated");
        assert_eq!(event.payload["repository_name"], "test-repo");
        assert_eq!(event.payload["branch"], "main");
        assert_eq!(event.payload["source_payload"]["sha"], "1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b");
        assert_eq!(event.payload["source_payload"]["message"], "Implement kernel config module");

        // Checkout Event log line
        let checkout_line = "1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b f0e9d8c7b6a5f4e3d2c1b0a9f8e7d6c5b4a3f2e1 User <email@example.com> 1672531205 +0000 checkout: moving from main to feature/observe";
        let checkout_event = GitEventNormalizer::normalize(path, "test-repo", "feature/observe", checkout_line).unwrap();
        
        assert_eq!(checkout_event.event_type, "GitBranchSwitched");
        assert_eq!(checkout_event.payload["source_payload"]["from_branch"], "main");
        assert_eq!(checkout_event.payload["source_payload"]["to_branch"], "feature/observe");
    }

    #[tokio::test]
    async fn test_git_polling_and_publisher() {
        let dir = tempdir().unwrap();
        let repo_path = dir.path().to_path_buf();
        let git_dir = repo_path.join(".git");
        std::fs::create_dir_all(&git_dir).unwrap();

        setup_mock_head(&git_dir, "ref: refs/heads/main\n");
        setup_mock_reflog(&git_dir, "0000000000000000000000000000000000000000 1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b User <email@example.com> 1672531199 +0000 commit: Initial commit\n");

        let registry = Arc::new(ServiceRegistry::new());
        let bus = Arc::new(MemoryEventBus::new(100));
        let config = Arc::new(ConfigurationService::new());
        let logger = ChronosLogger::new(LogContext::new());
        
        let adapter = GitAdapter::new(registry, bus.clone(), config, logger);
        let mut sub = bus.subscribe();

        adapter.watch_repository(repo_path.clone()).await.unwrap();
        
        // Drain the discovery event
        let _ = sub.next_event().await.unwrap();

        // Append a new reflog line (checkout)
        setup_mock_reflog(&git_dir, "0000000000000000000000000000000000000000 1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b User <email@example.com> 1672531199 +0000 commit: Initial commit
1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b f0e9d8c7b6a5f4e3d2c1b0a9f8e7d6c5b4a3f2e1 User <email@example.com> 1672531205 +0000 checkout: moving from main to dev\n");

        // Run adapter poll
        adapter.poll().await.unwrap();

        // We should receive the checkout event
        let next_evt = sub.next_event().await.unwrap();
        assert_eq!(next_evt.event_type, "GitBranchSwitched");
        assert_eq!(next_evt.payload["source_payload"]["from_branch"], "main");
        assert_eq!(next_evt.payload["source_payload"]["to_branch"], "dev");
    }
}
