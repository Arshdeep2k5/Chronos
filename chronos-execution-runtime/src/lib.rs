//! # Chronos Execution Runtime
//!
//! Layer 5 Execution engine responsible for executing continuation actions,
//! restoring context, and sending desktop or intervention notifications.

use async_trait::async_trait;
use chrono::Utc;
use chronos_bus::EventBus;
use chronos_core::{ChronosAction, ChronosEvent};
use chronos_logging::ChronosLogger;
#[cfg(test)]
use chronos_logging::LogContext;
use chronos_registry::{ServiceDescriptor, ServiceRegistry, ServiceType};
use serde_json::json;
use std::sync::Arc;

/// Errors that can occur during action execution.
#[derive(Debug, thiserror::Error)]
pub enum ExecutionError {
    #[error("Executor registry failure: {0}")]
    Registry(String),
    #[error("EventBus publish failure: {0}")]
    Bus(String),
    #[error("Action target error: {0}")]
    ActionTarget(String),
    #[error("Unsupported action type: {0}")]
    Unsupported(String),
}

/// Generic interface representing a specific executor component.
#[async_trait]
pub trait Executor: Send + Sync {
    async fn execute(&self, action: &ChronosAction) -> Result<serde_json::Value, ExecutionError>;
}

/// Handles reopening file lists and restoring active workspace layouts.
pub struct WorkspaceRestorationExecutor {
    logger: ChronosLogger,
}

impl WorkspaceRestorationExecutor {
    pub fn new(logger: ChronosLogger) -> Self {
        Self { logger }
    }
}

#[async_trait]
impl Executor for WorkspaceRestorationExecutor {
    async fn execute(&self, action: &ChronosAction) -> Result<serde_json::Value, ExecutionError> {
        let files = action.payload.get("files_to_reopen")
            .and_then(|v| v.as_array())
            .ok_or_else(|| ExecutionError::ActionTarget("Missing files_to_reopen array".to_string()))?;

        for file_val in files {
            if let Some(file_str) = file_val.as_str() {
                self.logger.info(&format!("[WorkspaceRestoration] Reopening file: {}", file_str), None);
                
                // Simulate physical file open (or write mock update log)
                // Windows-first: we could call std::process::Command to spawn notepad / editors,
                // but since this is a headless kernel service, logging and writing state changes is canonical.
            }
        }

        Ok(json!({ "restored_files_count": files.len() }))
    }
}

/// Handles writing project recovery plans and continuation plans.
pub struct RecoveryPlanExecutor {
    logger: ChronosLogger,
}

impl RecoveryPlanExecutor {
    pub fn new(logger: ChronosLogger) -> Self {
        Self { logger }
    }
}

#[async_trait]
impl Executor for RecoveryPlanExecutor {
    async fn execute(&self, action: &ChronosAction) -> Result<serde_json::Value, ExecutionError> {
        let next_action = action.payload.get("recommended_next_action")
            .and_then(|v| v.as_str())
            .unwrap_or("Review project tasks");

        self.logger.info(&format!("[RecoveryPlan] Materializing next recovery action: {}", next_action), None);

        Ok(json!({ "recovery_plan_status": "materialized", "recommended_action": next_action }))
    }
}

/// Handles showing OS desktop alerts or internal warnings.
pub struct NotificationExecutor {
    logger: ChronosLogger,
}

impl NotificationExecutor {
    pub fn new(logger: ChronosLogger) -> Self {
        Self { logger }
    }
}

#[async_trait]
impl Executor for NotificationExecutor {
    async fn execute(&self, action: &ChronosAction) -> Result<serde_json::Value, ExecutionError> {
        let digest = action.payload.get("digest")
            .and_then(|v| v.as_str())
            .unwrap_or("No details available");

        self.logger.info(&format!("[Notification] Displaying desktop alert: {}", digest), None);

        // Windows-first toast/notification mock triggers:
        // In full production app, this spawns windows Toast notification scripts or powershell balloon alerts.
        
        Ok(json!({ "notification_sent": true }))
    }
}

/// Main Action Dispatcher and Coordinator.
pub struct ActionExecutor {
    registry: Arc<ServiceRegistry>,
    bus: Arc<dyn EventBus>,
    logger: ChronosLogger,
    workspace_executor: WorkspaceRestorationExecutor,
    recovery_executor: RecoveryPlanExecutor,
    notify_executor: NotificationExecutor,
}

impl ActionExecutor {
    pub fn new(
        registry: Arc<ServiceRegistry>,
        bus: Arc<dyn EventBus>,
        logger: ChronosLogger,
    ) -> Self {
        Self {
            registry,
            bus,
            logger: logger.clone(),
            workspace_executor: WorkspaceRestorationExecutor::new(logger.clone()),
            recovery_executor: RecoveryPlanExecutor::new(logger.clone()),
            notify_executor: NotificationExecutor::new(logger),
        }
    }

    /// Registers the Execution Service capability with the Registry.
    pub async fn start(&self) -> Result<(), ExecutionError> {
        let desc = ServiceDescriptor::new(
            "chronos-execution-runtime",
            "Chronos Execution Runtime",
            ServiceType::Engine,
            "1.0.0",
            vec!["ExecuteActions".to_string()],
            vec![
                "WorkspaceRestoreRequest".to_string(),
                "RecoveryPlan".to_string(),
                "ContinuationPlan".to_string(),
            ],
            vec![
                "ActionStarted".to_string(),
                "ActionCompleted".to_string(),
                "ActionFailed".to_string(),
            ],
        );

        self.registry.register(desc)
            .await
            .map_err(|e| ExecutionError::Registry(e.to_string()))?;

        self.logger.info("Chronos Execution Runtime started and registered.", None);
        Ok(())
    }

    /// Executes a ChronosAction end-to-end, publishing started, completed, or failed events.
    pub async fn execute_action(&self, action: &ChronosAction) -> Result<(), ExecutionError> {
        // 1. Publish ActionStarted event
        let start_event = ChronosEvent::new(
            "ActionStarted",
            "ExecutionRuntime",
            json!({ "action_id": action.id, "action_type": action.action_type, "timestamp": Utc::now().to_rfc3339() }),
        );
        self.bus.publish(start_event)
            .map_err(|e| ExecutionError::Bus(e.to_string()))?;

        // 2. Dispatch to matching executor
        let outcome = match action.action_type.as_str() {
            "WorkspaceRestoreRequest" => self.workspace_executor.execute(action).await,
            "RecoveryPlan" => self.recovery_executor.execute(action).await,
            "ContinuationPlan" => self.notify_executor.execute(action).await,
            other => Err(ExecutionError::Unsupported(other.to_string())),
        };

        // 3. Publish Completion or Failure event
        match outcome {
            Ok(payload) => {
                let comp_event = ChronosEvent::new(
                    "ActionCompleted",
                    "ExecutionRuntime",
                    json!({
                        "action_id": action.id,
                        "action_type": action.action_type,
                        "timestamp": Utc::now().to_rfc3339(),
                        "outcome": payload,
                    }),
                );
                self.bus.publish(comp_event)
                    .map_err(|e| ExecutionError::Bus(e.to_string()))?;
                
                self.logger.info(&format!("Action {} executed successfully.", action.id), None);
                Ok(())
            }
            Err(err) => {
                let fail_event = ChronosEvent::new(
                    "ActionFailed",
                    "ExecutionRuntime",
                    json!({
                        "action_id": action.id,
                        "action_type": action.action_type,
                        "timestamp": Utc::now().to_rfc3339(),
                        "error": err.to_string(),
                    }),
                );
                self.bus.publish(fail_event)
                    .map_err(|e| ExecutionError::Bus(e.to_string()))?;
                
                self.logger.error(&format!("Action {} failed: {}", action.id, err), None);
                Err(err)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chronos_bus::MemoryEventBus;
    use tokio::time::Duration;

    #[tokio::test]
    async fn test_action_execution_flow() {
        let registry = Arc::new(ServiceRegistry::new());
        let bus = Arc::new(MemoryEventBus::new(100));
        let logger = ChronosLogger::new(LogContext::new());

        let orchestrator = ActionExecutor::new(registry, bus.clone(), logger);
        orchestrator.start().await.unwrap();

        let mut sub = bus.subscribe();

        // 1. Dispatch WorkspaceRestoreRequest
        let action = ChronosAction::new(
            "decision-1",
            "WorkspaceRestoreRequest",
            json!({ "files_to_reopen": ["src/lib.rs", "Cargo.toml"] }),
        );

        orchestrator.execute_action(&action).await.unwrap();

        let mut started = false;
        let mut completed = false;

        // Try reading twice with a timeout
        for _ in 0..2 {
            if let Ok(Ok(event)) = tokio::time::timeout(Duration::from_millis(100), sub.next_event()).await {
                if event.event_type == "ActionStarted" {
                    started = true;
                }
                if event.event_type == "ActionCompleted" {
                    completed = true;
                }
            }
        }

        assert!(started);
        assert!(completed);
    }

    #[tokio::test]
    async fn test_action_execution_failure() {
        let registry = Arc::new(ServiceRegistry::new());
        let bus = Arc::new(MemoryEventBus::new(100));
        let logger = ChronosLogger::new(LogContext::new());

        let orchestrator = ActionExecutor::new(registry, bus.clone(), logger);
        let mut sub = bus.subscribe();

        // Dispatch unsupported action type to trigger failure
        let action = ChronosAction::new("decision-2", "InvalidActionType", json!({}));
        let res = orchestrator.execute_action(&action).await;
        
        assert!(res.is_err());

        let mut failed = false;
        for _ in 0..2 {
            if let Ok(Ok(event)) = tokio::time::timeout(Duration::from_millis(100), sub.next_event()).await {
                if event.event_type == "ActionFailed" {
                    failed = true;
                }
            }
        }

        assert!(failed);
    }
}
