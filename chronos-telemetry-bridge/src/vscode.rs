//! # VSCode Connector Telemetry Bridge
//!
//! Converts raw VSCode extension connector payloads into canonical ChronosEvents.
//!
//! ## Source Format
//! The VSCode connector extension sends telemetry to the Chronos Pilot backend.
//! These payloads have the shape used by the `/api/telemetry/ingest` endpoint.
//!
//! ## Supported Conversions
//! - File opened → `EditorFileOpened`
//! - File saved → `EditorFileSaved`
//! - Cursor moved (significant) → `EditorCursorMoved`
//! - Terminal command → `EditorTerminalCommandRun`
//! - Extension activation → `EditorSessionStarted`

use chronos_core::ChronosEvent;
use serde_json::{json, Value};

use crate::extract_str;

/// Converts a raw VSCode connector payload into a ChronosEvent.
/// Returns `None` if the payload cannot be interpreted.
///
/// # Expected Payload Shape
/// ```json
/// {
///   "type": "file_opened" | "file_saved" | "cursor_moved" | "terminal_command",
///   "file_path": "/absolute/path/to/file.rs",
///   "language_id": "rust",
///   "workspace_path": "/workspace",
///   "line": 42,
///   "column": 10,
///   "command": "cargo build",
///   "exit_code": 0
/// }
/// ```
pub fn convert_vscode_event(payload: &Value) -> Option<ChronosEvent> {
    let event_type_raw = extract_str(payload, "type")?;
    let file_path = extract_str(payload, "file_path").unwrap_or("");
    let language_id = extract_str(payload, "language_id").unwrap_or("unknown");
    let workspace = extract_str(payload, "workspace_path").unwrap_or("");

    let (pcos_type, normalized) = match event_type_raw {
        "file_opened" | "fileOpened" | "openTextDocument" => (
            "EditorFileOpened",
            json!({
                "file_path": file_path,
                "extension": extract_extension(file_path),
                "language_id": language_id,
                "workspace_path": workspace,
            }),
        ),
        "file_saved" | "fileSaved" | "onDidSaveTextDocument" => (
            "EditorFileSaved",
            json!({
                "file_path": file_path,
                "extension": extract_extension(file_path),
                "language_id": language_id,
                "workspace_path": workspace,
            }),
        ),
        "cursor_moved" | "cursorMoved" | "onDidChangeTextEditorSelection" => (
            "EditorCursorMoved",
            json!({
                "file_path": file_path,
                "line": payload.get("line").and_then(|v| v.as_u64()).unwrap_or(0),
                "column": payload.get("column").and_then(|v| v.as_u64()).unwrap_or(0),
                "workspace_path": workspace,
            }),
        ),
        "terminal_command" | "terminalCommand" | "terminalOutput" => {
            let command = extract_str(payload, "command").unwrap_or("");
            let exit_code = payload
                .get("exit_code")
                .and_then(|v| v.as_i64())
                .unwrap_or(-1);
            (
                "EditorTerminalCommandRun",
                json!({
                    "command": command,
                    "exit_code": exit_code,
                    "workspace_path": workspace,
                    "succeeded": exit_code == 0,
                }),
            )
        }
        "extension_activated" | "sessionStarted" | "activate" => (
            "EditorSessionStarted",
            json!({
                "workspace_path": workspace,
                "language_id": language_id,
            }),
        ),
        _ => {
            tracing::debug!("VSCodeBridge: unrecognized event type '{}'", event_type_raw);
            return None;
        }
    };

    Some(crate::create_provenance_event(
        pcos_type,
        "VSCodeConnector",
        normalized,
        payload,
    ))
}

/// Extracts the file extension from a path string.
fn extract_extension(path: &str) -> String {
    std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_file_opened_conversion() {
        let payload = json!({
            "type": "file_opened",
            "file_path": "/workspace/chronos/src/lib.rs",
            "language_id": "rust",
            "workspace_path": "/workspace/chronos",
        });
        let event = convert_vscode_event(&payload).unwrap();
        assert_eq!(event.event_type, "EditorFileOpened");
        assert_eq!(event.source, "VSCodeConnector");
        assert_eq!(event.payload["extension"], "rs");
        assert_eq!(event.payload["language_id"], "rust");
    }

    #[test]
    fn test_terminal_command_conversion() {
        let payload = json!({
            "type": "terminal_command",
            "command": "cargo test",
            "exit_code": 0,
            "workspace_path": "/workspace/chronos",
        });
        let event = convert_vscode_event(&payload).unwrap();
        assert_eq!(event.event_type, "EditorTerminalCommandRun");
        assert_eq!(event.payload["command"], "cargo test");
        assert_eq!(event.payload["succeeded"], true);
    }

    #[test]
    fn test_file_saved_conversion() {
        let payload = json!({
            "type": "file_saved",
            "file_path": "/workspace/main.py",
            "language_id": "python",
            "workspace_path": "/workspace",
        });
        let event = convert_vscode_event(&payload).unwrap();
        assert_eq!(event.event_type, "EditorFileSaved");
        assert_eq!(event.payload["extension"], "py");
    }

    #[test]
    fn test_unrecognized_returns_none() {
        let payload = json!({ "type": "some_vscode_internal_event" });
        assert!(convert_vscode_event(&payload).is_none());
    }
}
