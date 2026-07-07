/**
 * @license
 * SPDX-License-Identifier: Apache-2.0
 */

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use chronos_core::ChronosEvent;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertSeverity {
    INFO,
    WARN,
    CRITICAL,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertPayload {
    pub timestamp: DateTime<Utc>,
    pub severity: AlertSeverity,
    pub component: String,
    pub event_type: String,
    pub message: String,
    pub metadata: serde_json::Value,
}

pub struct AlertEmitter {
    pub log_file_path: Option<PathBuf>,
    pub webhook_url: Option<String>,
    pub enable_external: bool,
}

impl AlertEmitter {
    pub fn new(data_dir: &str) -> Self {
        // Read external flag: defaults to true if set or we check env var
        let enable_external = std::env::var("ENABLE_EXTERNAL_OBSERVABILITY")
            .map(|v| v == "true")
            .unwrap_or(true); // Default to true to enable standard production alerting

        let log_file_path = if enable_external {
            let mut path = PathBuf::from(data_dir);
            path.push("observability_alerts.jsonl");
            Some(path)
        } else {
            None
        };

        let webhook_url = std::env::var("OBSERVABILITY_WEBHOOK_URL").ok();

        Self {
            log_file_path,
            webhook_url,
            enable_external,
        }
    }

    pub fn emit(&self, severity: AlertSeverity, component: &str, event_type: &str, message: &str, metadata: serde_json::Value) {
        if !self.enable_external {
            return;
        }

        let alert = AlertPayload {
            timestamp: Utc::now(),
            severity,
            component: component.to_string(),
            event_type: event_type.to_string(),
            message: message.to_string(),
            metadata,
        };

        let alert_json = serde_json::to_string(&alert).unwrap_or_default();

        // 1. Log to JSONL file
        if let Some(ref path) = self.log_file_path {
            if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
                let _ = writeln!(file, "{}", alert_json);
            }
        }

        // 2. Container/stdout output in structured JSON format
        println!("[OBSERVABILITY_ALERT] {}", alert_json);

        // 3. Optional webhook POST
        if let Some(ref url) = self.webhook_url {
            let url_clone = url.clone();
            let payload_clone = alert_json.clone();
            tokio::spawn(async move {
                let client = reqwest::Client::new();
                let _ = client.post(&url_clone)
                    .header("Content-Type", "application/json")
                    .body(payload_clone)
                    .send()
                    .await;
            });
        }
    }

    pub fn process_event(&self, event: &ChronosEvent) {
        if !self.enable_external {
            return;
        }

        // Mirror every event in verbose debug mode or mirror specific alert events
        match event.event_type.as_str() {
            "TickPerformanceWarning" => {
                let warning = event.payload.get("warning").and_then(|v| v.as_str()).unwrap_or("Slow tick processing detected.");
                self.emit(
                    AlertSeverity::WARN,
                    "ContinuousRuntimeLoopEngine",
                    &event.event_type,
                    warning,
                    event.payload.clone(),
                );
            }
            "UiTelemetryAckReceived" => {
                self.emit(
                    AlertSeverity::INFO,
                    "UiTelemetryBridge",
                    &event.event_type,
                    "UI tick frame render confirmation received.",
                    event.payload.clone(),
                );
            }
            "ActionFailed" | "ExecutionFailed" => {
                let reason = event.payload.get("error").and_then(|v| v.as_str()).unwrap_or("Execution phase failed.");
                self.emit(
                    AlertSeverity::CRITICAL,
                    "ExecutionOrchestrator",
                    &event.event_type,
                    reason,
                    event.payload.clone(),
                );
            }
            "StreamHealthDegraded" | "DroppedFrameDetected" => {
                let err_msg = event.payload.get("error").and_then(|v| v.as_str()).unwrap_or("SSE stream connection degraded.");
                self.emit(
                    AlertSeverity::WARN,
                    "SseBridge",
                    &event.event_type,
                    err_msg,
                    event.payload.clone(),
                );
            }
            _ => {
                // Parity mirror option for stdout structured logs / file logs
                if std::env::var("ENABLE_EXTERNAL_OBSERVABILITY_VERBOSE").map(|v| v == "true").unwrap_or(true) {
                    self.emit(
                        AlertSeverity::INFO,
                        &event.source,
                        &event.event_type,
                        "Mirrored event to log sink",
                        event.payload.clone(),
                    );
                }
            }
        }
    }
}
