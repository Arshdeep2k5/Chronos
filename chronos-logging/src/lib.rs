//! # Chronos Logging
//!
//! Provides unified structured observability across the entire Chronos platform.
//! Subsystems must emit logs through this crate rather than directly via `println` or `tracing`.
//!
//! # Operational Profile
//! * **Consumes:** Internal events, errors, status updates, context data.
//! * **Produces:** Structured JSON or Human-readable terminal output via `tracing-subscriber`.
//! * **Capabilities:** Context propagation, structured fields, thread-safe asynchronous logging.
//! * **Dependencies:** `tracing`, `tracing-subscriber`, `serde_json`.
//! * **Failure Modes:** Initialization failures (if called twice), serialization errors on custom fields.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, error, info, trace, warn};
use tracing_subscriber::{fmt, EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

/// Defines the output format for the logger.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Json,
    HumanReadable,
}

/// Core context attached to every log event.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LogContext {
    pub subsystem_id: Option<String>,
    pub correlation_id: Option<String>,
    pub session_id: Option<String>,
    pub event_id: Option<String>,
    pub project_id: Option<String>,
    pub commitment_id: Option<String>,
    /// Additional unstructured fields that a subsystem might want to attach.
    #[serde(flatten)]
    pub custom_fields: HashMap<String, serde_json::Value>,
}

impl LogContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_subsystem(mut self, id: impl Into<String>) -> Self {
        self.subsystem_id = Some(id.into());
        self
    }

    pub fn with_correlation(mut self, id: impl Into<String>) -> Self {
        self.correlation_id = Some(id.into());
        self
    }

    pub fn with_session(mut self, id: impl Into<String>) -> Self {
        self.session_id = Some(id.into());
        self
    }

    pub fn with_field(mut self, key: impl Into<String>, value: impl Serialize) -> Self {
        if let Ok(val) = serde_json::to_value(value) {
            self.custom_fields.insert(key.into(), val);
        }
        self
    }
}

/// Represents the structured log payload (primarily for serialization/testing purposes).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredLogEvent {
    pub level: String,
    pub timestamp: DateTime<Utc>,
    pub message: String,
    pub context: LogContext,
}

/// The main logger instance. This struct provides the unified interface 
/// for emitting structured logs across the platform.
#[derive(Clone)]
pub struct ChronosLogger {
    default_context: LogContext,
}

impl ChronosLogger {
    /// Initializes the global tracing subscriber. This should be called exactly once 
    /// at the start of the application (Layer 0 Daemon).
    pub fn init_global(format: OutputFormat, default_level: &str) -> Result<(), Box<dyn std::error::Error>> {
        let filter = EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new(default_level));

        match format {
            OutputFormat::Json => {
                tracing_subscriber::registry()
                    .with(filter)
                    .with(fmt::layer().json().flatten_event(true))
                    .try_init()?;
            }
            OutputFormat::HumanReadable => {
                tracing_subscriber::registry()
                    .with(filter)
                    .with(fmt::layer().compact())
                    .try_init()?;
            }
        }
        Ok(())
    }

    /// Creates a new ChronosLogger bound to a specific initial context.
    pub fn new(default_context: LogContext) -> Self {
        Self { default_context }
    }

    /// Creates a derived logger with merged context.
    pub fn with_context(&self, extra_context: LogContext) -> Self {
        let mut new_context = self.default_context.clone();
        
        if extra_context.subsystem_id.is_some() { new_context.subsystem_id = extra_context.subsystem_id; }
        if extra_context.correlation_id.is_some() { new_context.correlation_id = extra_context.correlation_id; }
        if extra_context.session_id.is_some() { new_context.session_id = extra_context.session_id; }
        if extra_context.event_id.is_some() { new_context.event_id = extra_context.event_id; }
        if extra_context.project_id.is_some() { new_context.project_id = extra_context.project_id; }
        if extra_context.commitment_id.is_some() { new_context.commitment_id = extra_context.commitment_id; }
        
        for (k, v) in extra_context.custom_fields {
            new_context.custom_fields.insert(k, v);
        }

        Self { default_context: new_context }
    }

    /// Helper to serialize custom fields so they can be logged cleanly by tracing.
    fn format_custom(ctx: &LogContext) -> String {
        serde_json::to_string(&ctx.custom_fields).unwrap_or_else(|_| "{}".to_string())
    }

    pub fn trace(&self, message: &str, mut ctx: Option<LogContext>) {
        let context = ctx.take().unwrap_or_else(|| self.default_context.clone());
        trace!(
            subsystem = context.subsystem_id.as_deref(),
            correlation = context.correlation_id.as_deref(),
            session = context.session_id.as_deref(),
            event = context.event_id.as_deref(),
            project = context.project_id.as_deref(),
            commitment = context.commitment_id.as_deref(),
            custom = Self::format_custom(&context),
            "{}", message
        );
    }

    pub fn debug(&self, message: &str, mut ctx: Option<LogContext>) {
        let context = ctx.take().unwrap_or_else(|| self.default_context.clone());
        debug!(
            subsystem = context.subsystem_id.as_deref(),
            correlation = context.correlation_id.as_deref(),
            session = context.session_id.as_deref(),
            event = context.event_id.as_deref(),
            project = context.project_id.as_deref(),
            commitment = context.commitment_id.as_deref(),
            custom = Self::format_custom(&context),
            "{}", message
        );
    }

    pub fn info(&self, message: &str, mut ctx: Option<LogContext>) {
        let context = ctx.take().unwrap_or_else(|| self.default_context.clone());
        info!(
            subsystem = context.subsystem_id.as_deref(),
            correlation = context.correlation_id.as_deref(),
            session = context.session_id.as_deref(),
            event = context.event_id.as_deref(),
            project = context.project_id.as_deref(),
            commitment = context.commitment_id.as_deref(),
            custom = Self::format_custom(&context),
            "{}", message
        );
    }

    pub fn warn(&self, message: &str, mut ctx: Option<LogContext>) {
        let context = ctx.take().unwrap_or_else(|| self.default_context.clone());
        warn!(
            subsystem = context.subsystem_id.as_deref(),
            correlation = context.correlation_id.as_deref(),
            session = context.session_id.as_deref(),
            event = context.event_id.as_deref(),
            project = context.project_id.as_deref(),
            commitment = context.commitment_id.as_deref(),
            custom = Self::format_custom(&context),
            "{}", message
        );
    }

    pub fn error(&self, message: &str, mut ctx: Option<LogContext>) {
        let context = ctx.take().unwrap_or_else(|| self.default_context.clone());
        error!(
            subsystem = context.subsystem_id.as_deref(),
            correlation = context.correlation_id.as_deref(),
            session = context.session_id.as_deref(),
            event = context.event_id.as_deref(),
            project = context.project_id.as_deref(),
            commitment = context.commitment_id.as_deref(),
            custom = Self::format_custom(&context),
            "{}", message
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Since `tracing` global initialization can only happen once per process,
    // we use a single thread safe test or avoid double initialization.
    
    #[test]
    fn test_log_context_builder() {
        let ctx = LogContext::new()
            .with_subsystem("kernel")
            .with_correlation("corr-123")
            .with_field("cpu_usage", 45);

        assert_eq!(ctx.subsystem_id.as_deref(), Some("kernel"));
        assert_eq!(ctx.correlation_id.as_deref(), Some("corr-123"));
        assert_eq!(ctx.custom_fields.get("cpu_usage").and_then(|v| v.as_i64()), Some(45));
    }

    #[test]
    fn test_logger_derivation() {
        let base_ctx = LogContext::new().with_subsystem("kernel");
        let logger = ChronosLogger::new(base_ctx);
        
        let derived_ctx = LogContext::new().with_session("sess-abc");
        let derived_logger = logger.with_context(derived_ctx);
        
        assert_eq!(derived_logger.default_context.subsystem_id.as_deref(), Some("kernel"));
        assert_eq!(derived_logger.default_context.session_id.as_deref(), Some("sess-abc"));
    }
}
