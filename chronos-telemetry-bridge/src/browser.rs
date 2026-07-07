//! # Browser Extension Telemetry Bridge
//!
//! Converts raw browser extension payloads into canonical ChronosEvents.
//!
//! ## Source Format
//! The MV3 browser extension sends WebSocket messages to the existing Chronos
//! Pilot backend. These payloads have the shape used by the `/api/telemetry/ingest`
//! endpoint in `server.rs`. This bridge normalizes them into the PCOS event schema.
//!
//! ## Supported Conversions
//! - Tab activation → `BrowserTabActivated`
//! - URL change → `BrowserUrlChanged`
//! - Page load → `BrowserPageLoaded`
//! - Tab close → `BrowserTabClosed`

use chronos_core::ChronosEvent;
use serde_json::{json, Value};

use crate::extract_str;

/// Converts a raw browser extension WebSocket payload into a ChronosEvent.
/// Returns `None` if the payload cannot be interpreted.
///
/// # Expected Payload Shape
/// ```json
/// {
///   "type": "tab_activated" | "url_changed" | "page_loaded" | "tab_closed",
///   "url": "https://...",
///   "title": "Page Title",
///   "tab_id": 123,
///   "window_id": 456,
///   "timestamp": "2024-01-01T00:00:00Z"
/// }
/// ```
pub fn convert_browser_event(payload: &Value) -> Option<ChronosEvent> {
    let event_type_raw = extract_str(payload, "type")?;
    let url = extract_str(payload, "url").unwrap_or("");
    let title = extract_str(payload, "title").unwrap_or("unknown");
    let tab_id = payload.get("tab_id").and_then(|v| v.as_u64()).unwrap_or(0);

    let (pcos_type, normalized_payload) = match event_type_raw {
        "tab_activated" | "tabActivated" => (
            "BrowserTabActivated",
            json!({
                "url": url,
                "title": title,
                "tab_id": tab_id,
                "domain": extract_domain(url),
            }),
        ),
        "url_changed" | "urlChanged" | "navigation" => (
            "BrowserUrlChanged",
            json!({
                "url": url,
                "title": title,
                "tab_id": tab_id,
                "domain": extract_domain(url),
            }),
        ),
        "page_loaded" | "pageLoad" | "domContentLoaded" => (
            "BrowserPageLoaded",
            json!({
                "url": url,
                "title": title,
                "tab_id": tab_id,
                "domain": extract_domain(url),
            }),
        ),
        "tab_closed" | "tabClosed" | "tabRemoved" => (
            "BrowserTabClosed",
            json!({
                "tab_id": tab_id,
            }),
        ),
        _ => {
            tracing::debug!("BrowserBridge: unrecognized event type '{}'", event_type_raw);
            return None;
        }
    };

    Some(crate::create_provenance_event(
        pcos_type,
        "BrowserExtension",
        normalized_payload,
        payload,
    ))
}

/// Extracts the domain from a URL string (best-effort, no regex dependency).
fn extract_domain(url: &str) -> String {
    // Strip protocol
    let without_proto = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url);
    // Take up to the first '/'
    without_proto
        .split('/')
        .next()
        .unwrap_or(without_proto)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_tab_activated_conversion() {
        let payload = json!({
            "type": "tab_activated",
            "url": "https://github.com/user/chronos",
            "title": "Chronos on GitHub",
            "tab_id": 42,
        });
        let event = convert_browser_event(&payload).unwrap();
        assert_eq!(event.event_type, "BrowserTabActivated");
        assert_eq!(event.source, "BrowserExtension");
        assert_eq!(event.payload["domain"], "github.com");
        assert_eq!(event.payload["tab_id"], 42);
    }

    #[test]
    fn test_url_changed_conversion() {
        let payload = json!({
            "type": "url_changed",
            "url": "https://docs.rs/axum",
            "title": "axum docs",
            "tab_id": 1,
        });
        let event = convert_browser_event(&payload).unwrap();
        assert_eq!(event.event_type, "BrowserUrlChanged");
        assert_eq!(event.payload["domain"], "docs.rs");
    }

    #[test]
    fn test_unrecognized_type_returns_none() {
        let payload = json!({ "type": "unknown_event" });
        assert!(convert_browser_event(&payload).is_none());
    }

    #[test]
    fn test_missing_type_returns_none() {
        let payload = json!({ "url": "https://example.com" });
        assert!(convert_browser_event(&payload).is_none());
    }

    #[test]
    fn test_domain_extraction() {
        assert_eq!(extract_domain("https://github.com/user/repo"), "github.com");
        assert_eq!(extract_domain("http://localhost:3000/api"), "localhost:3000");
        assert_eq!(extract_domain("github.com"), "github.com");
    }
}
