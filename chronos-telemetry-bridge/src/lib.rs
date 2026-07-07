//! # Chronos Telemetry Bridge
//!
//! Converts incoming raw telemetry signals from the existing Chronos Pilot
//! data sources into canonical `ChronosEvent`s and publishes them onto the
//! Cognitive Bus.
//!
//! ## Bridged Sources
//!
//! | Source                    | Raw Format      | ChronosEvent Type               |
//! |---------------------------|-----------------|---------------------------------|
//! | Browser Extension         | JSON telemetry  | `BrowserTabActivated`           |
//! | Browser Extension         | JSON telemetry  | `BrowserUrlChanged`             |
//! | VSCode Connector          | JSON payload    | `EditorFileOpened`              |
//! | VSCode Connector          | JSON payload    | `EditorCursorMoved`             |
//! | Manual Ingestion API      | context_node    | `ManualContextIngested`         |
//! | Telemetry DB (legacy)     | telemetry_log   | `TelemetryLogIngested`          |
//!
//! ## Architecture
//! - Observe incoming raw payload → Normalize → Publish to EventBus
//! - No reasoning, no state mutation, no AI inference
//! - Each conversion function is pure and deterministic
//! - Replay-safe: duplicate suppression uses content hash

pub mod browser;
pub mod vscode;
pub mod manual;

use chronos_bus::EventBus;
use chronos_core::ChronosEvent;
use serde_json::Value;

/// The result of a telemetry conversion.
#[derive(Debug)]
pub enum BridgeResult {
    /// Successfully converted and published a ChronosEvent.
    Published(ChronosEvent),
    /// The incoming payload was valid but represented a duplicate (suppressed).
    Duplicate,
    /// The incoming payload was unrecognizable or malformed.
    Skipped(String),
}

/// Publishes a ChronosEvent to the Cognitive Bus and logs the outcome.
pub fn publish_event(bus: &dyn EventBus, event: ChronosEvent) -> BridgeResult {
    tracing::debug!(
        "TelemetryBridge: publishing event type={}",
        event.event_type
    );
    match bus.publish(event.clone()) {
        Ok(_) => BridgeResult::Published(event),
        Err(e) => BridgeResult::Skipped(format!("Bus publish error: {:?}", e)),
    }
}

/// Extracts a string field from a JSON value, returning None if absent or not a string.
pub fn extract_str<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key)?.as_str()
}

/// Helper to create a ChronosEvent from converted telemetry, extracting and preserving
/// original event ID and timestamp from the raw input payload if they exist.
pub fn create_provenance_event(
    event_type: impl Into<String>,
    source: impl Into<String>,
    payload: serde_json::Value,
    raw_payload: &Value,
) -> ChronosEvent {
    use chrono::Utc;
    use uuid::Uuid;

    // 1. Try to extract original event ID
    let original_id = raw_payload.get("id")
        .or_else(|| raw_payload.get("event_id"))
        .or_else(|| raw_payload.get("uuid"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // 2. Try to extract original timestamp
    let original_timestamp = raw_payload.get("timestamp")
        .or_else(|| raw_payload.get("created_at"))
        .or_else(|| raw_payload.get("occurred_at"))
        .and_then(|v| {
            if let Some(s) = v.as_str() {
                // Try parsing ISO8601 string
                chrono::DateTime::parse_from_rfc3339(s)
                    .map(|dt| dt.with_timezone(&Utc))
                    .ok()
            } else if let Some(i) = v.as_i64() {
                // Try parsing epoch ms/seconds
                chrono::DateTime::from_timestamp_millis(i)
                    .or_else(|| chrono::DateTime::from_timestamp(i, 0))
            } else {
                None
            }
        });

    match (original_id, original_timestamp) {
        (Some(id), Some(ts)) => ChronosEvent::new_with_provenance(id, ts, event_type, source, payload),
        (Some(id), None) => ChronosEvent::new_with_provenance(id, Utc::now(), event_type, source, payload),
        (None, Some(ts)) => ChronosEvent::new_with_provenance(Uuid::new_v4().to_string(), ts, event_type, source, payload),
        (None, None) => ChronosEvent::new(event_type, source, payload),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_provenance_id_preservation() {
        let raw = json!({
            "id": "fixed-id-1234",
            "type": "tab_activated",
            "url": "https://google.com"
        });
        let event = browser::convert_browser_event(&raw).unwrap();
        assert_eq!(event.id, "fixed-id-1234");
    }

    #[test]
    fn test_provenance_timestamp_preservation() {
        let raw = json!({
            "timestamp": "2026-06-28T12:00:00Z",
            "type": "file_opened",
            "file_path": "main.rs"
        });
        let event = vscode::convert_vscode_event(&raw).unwrap();
        assert_eq!(event.timestamp.to_rfc3339(), "2026-06-28T12:00:00+00:00");
    }

    #[test]
    fn test_mixed_telemetry_provenance() {
        // VSCode payload
        let vscode_raw = json!({
            "uuid": "vscode-uuid",
            "created_at": "2026-06-28T12:30:00Z",
            "type": "cursor_moved",
            "file_path": "lib.rs",
            "line": 10,
            "column": 5
        });
        let vscode_event = vscode::convert_vscode_event(&vscode_raw).unwrap();
        assert_eq!(vscode_event.id, "vscode-uuid");
        assert_eq!(vscode_event.timestamp.to_rfc3339(), "2026-06-28T12:30:00+00:00");

        // Manual context node
        let manual_raw = json!({
            "event_id": "manual-event-id",
            "occurred_at": "2026-06-28T13:00:00Z",
            "entity_key": "COMMIT:xyz987",
            "display_name": "feat: init"
        });
        let manual_event = manual::convert_manual_ingestion(&manual_raw).unwrap();
        assert_eq!(manual_event.id, "manual-event-id");
        assert_eq!(manual_event.timestamp.to_rfc3339(), "2026-06-28T13:00:00+00:00");
    }
}

