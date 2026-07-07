//! # Manual Ingestion Bridge
//!
//! Converts raw manual context ingestion payloads (as sent to
//! `/api/telemetry/ingest` in the existing Chronos Pilot server.rs)
//! into canonical ChronosEvents.
//!
//! ## Source Format
//! The manual ingestion endpoint accepts `context_node` JSON objects
//! from the existing monolith. This bridge normalizes them into PCOS events.
//!
//! ## Supported Conversions
//! - context_node with entity_key starting with "COMMIT:" → `GitCommitCreated`
//! - context_node with entity_key starting with "FILE:" → `FileModified`
//! - context_node with entity_key starting with "URL:" → `BrowserUrlChanged`
//! - context_node with entity_key starting with "APP:" → `ApplicationActivated`
//! - context_node with entity_key starting with "REPO:" → `GitRepositoryDiscovered`
//! - Raw telemetry_log → `TelemetryLogIngested`
//! - Unclassified context → `ManualContextIngested`

use chronos_core::ChronosEvent;
use serde_json::{json, Value};

use crate::extract_str;

/// Converts a raw context_node or telemetry_log payload into a ChronosEvent.
/// Returns `None` if the payload cannot be classified.
///
/// # Expected Payload Shape (context_node)
/// ```json
/// {
///   "entity_key": "COMMIT:abc123",
///   "display_name": "Human readable label",
///   "raw_data": { ... },
///   "project_id": "uuid",
///   "node_type": "commit" | "file" | "url" | "app" | "repo"
/// }
/// ```
pub fn convert_manual_ingestion(payload: &Value) -> Option<ChronosEvent> {
    // Path A: context_node with entity_key
    if let Some(entity_key) = extract_str(payload, "entity_key") {
        return convert_context_node(entity_key, payload);
    }

    // Path B: raw telemetry_log with event_type field
    if let Some(event_type) = extract_str(payload, "event_type") {
        return Some(crate::create_provenance_event(
            "TelemetryLogIngested",
            "ManualIngestion",
            json!({
                "original_event_type": event_type,
                "raw": payload,
            }),
            payload,
        ));
    }

    // Path C: unclassified — wrap as a ManualContextIngested observation
    if let Some(display_name) = extract_str(payload, "display_name") {
        return Some(crate::create_provenance_event(
            "ManualContextIngested",
            "ManualIngestion",
            json!({
                "display_name": display_name,
                "raw": payload,
            }),
            payload,
        ));
    }

    None
}

/// Converts a context_node by inspecting its entity_key prefix.
fn convert_context_node(entity_key: &str, payload: &Value) -> Option<ChronosEvent> {
    let display_name = extract_str(payload, "display_name").unwrap_or("");
    let raw_data = payload.get("raw_data").cloned().unwrap_or(Value::Null);
    let project_id = extract_str(payload, "project_id").unwrap_or("");

    if let Some(hash) = entity_key.strip_prefix("COMMIT:") {
        return Some(crate::create_provenance_event(
            "GitCommitCreated",
            "ManualIngestion",
            json!({
                "commit_hash": hash,
                "message": display_name,
                "project_id": project_id,
                "source_payload": raw_data,
                "repository_path": extract_str(payload, "repository_path").unwrap_or(""),
            }),
            payload,
        ));
    }

    if let Some(path) = entity_key.strip_prefix("FILE:") {
        return Some(crate::create_provenance_event(
            "FileModified",
            "ManualIngestion",
            json!({
                "path": path,
                "extension": std::path::Path::new(path)
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or(""),
                "display_name": display_name,
                "project_id": project_id,
            }),
            payload,
        ));
    }

    if let Some(url) = entity_key.strip_prefix("URL:") {
        return Some(crate::create_provenance_event(
            "BrowserUrlChanged",
            "ManualIngestion",
            json!({
                "url": url,
                "title": display_name,
                "domain": url.split('/').next().unwrap_or(url),
            }),
            payload,
        ));
    }

    if let Some(app_name) = entity_key.strip_prefix("APP:") {
        return Some(crate::create_provenance_event(
            "ApplicationActivated",
            "ManualIngestion",
            json!({
                "process_name": app_name,
                "display_name": display_name,
            }),
            payload,
        ));
    }

    if let Some(repo_path) = entity_key.strip_prefix("REPO:") {
        return Some(crate::create_provenance_event(
            "GitRepositoryDiscovered",
            "ManualIngestion",
            json!({
                "repository_path": repo_path,
                "display_name": display_name,
                "project_id": project_id,
            }),
            payload,
        ));
    }

    // Fallback: emit as a generic manual ingestion event
    Some(crate::create_provenance_event(
        "ManualContextIngested",
        "ManualIngestion",
        json!({
            "entity_key": entity_key,
            "display_name": display_name,
            "raw": payload,
        }),
        payload,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_commit_node_conversion() {
        let payload = json!({
            "entity_key": "COMMIT:abc123def456",
            "display_name": "feat: add PCM engine",
            "project_id": "proj-001",
            "repository_path": "/workspace/chronos",
        });
        let event = convert_manual_ingestion(&payload).unwrap();
        assert_eq!(event.event_type, "GitCommitCreated");
        assert_eq!(event.payload["commit_hash"], "abc123def456");
        assert_eq!(event.payload["message"], "feat: add PCM engine");
    }

    #[test]
    fn test_file_node_conversion() {
        let payload = json!({
            "entity_key": "FILE:/workspace/chronos/src/lib.rs",
            "display_name": "lib.rs",
            "project_id": "proj-001",
        });
        let event = convert_manual_ingestion(&payload).unwrap();
        assert_eq!(event.event_type, "FileModified");
        assert_eq!(event.payload["path"], "/workspace/chronos/src/lib.rs");
        assert_eq!(event.payload["extension"], "rs");
    }

    #[test]
    fn test_repo_node_conversion() {
        let payload = json!({
            "entity_key": "REPO:/workspace/chronos",
            "display_name": "chronos",
            "project_id": "proj-001",
        });
        let event = convert_manual_ingestion(&payload).unwrap();
        assert_eq!(event.event_type, "GitRepositoryDiscovered");
        assert_eq!(event.payload["repository_path"], "/workspace/chronos");
    }

    #[test]
    fn test_url_node_conversion() {
        let payload = json!({
            "entity_key": "URL:https://docs.rs/axum",
            "display_name": "Axum Docs",
        });
        let event = convert_manual_ingestion(&payload).unwrap();
        assert_eq!(event.event_type, "BrowserUrlChanged");
        assert_eq!(event.payload["url"], "https://docs.rs/axum");
    }

    #[test]
    fn test_app_node_conversion() {
        let payload = json!({
            "entity_key": "APP:Code.exe",
            "display_name": "Visual Studio Code",
        });
        let event = convert_manual_ingestion(&payload).unwrap();
        assert_eq!(event.event_type, "ApplicationActivated");
        assert_eq!(event.payload["process_name"], "Code.exe");
    }

    #[test]
    fn test_unknown_payload_returns_none() {
        let payload = json!({ "something": "unrelated" });
        assert!(convert_manual_ingestion(&payload).is_none());
    }

    #[test]
    fn test_generic_manual_ingestion() {
        let payload = json!({
            "display_name": "Some manual note about project",
        });
        let event = convert_manual_ingestion(&payload).unwrap();
        assert_eq!(event.event_type, "ManualContextIngested");
    }
}
