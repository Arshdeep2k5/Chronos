//! # Axum Router
//!
//! Assembles all API handlers into the Axum router and applies CORS middleware.

use axum::{
    Router,
    routing::{get, post},
};
use tower_http::cors::{Any, CorsLayer};

use crate::{handlers, state::BridgeState};

/// Builds the complete API router with all PCOS endpoints and CORS headers.
pub fn build_router(state: BridgeState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        // ── Events ────────────────────────────────────────────────────────
        .route("/api/events/stream", get(handlers::handle_events_stream))
        .route("/api/events/stream/live", get(handlers::handle_events_stream_live))
        // ── State ─────────────────────────────────────────────────────────
        .route("/api/state", get(handlers::handle_state))
        .route("/api/session/current", get(handlers::handle_session_current))
        // ── Reasoning ─────────────────────────────────────────────────────
        .route(
            "/api/reasoning/forecasts",
            get(handlers::handle_reasoning_forecasts),
        )
        .route(
            "/api/reasoning/diagnostics",
            get(handlers::handle_reasoning_diagnostics),
        )
        // ── Execution ─────────────────────────────────────────────────────
        .route(
            "/api/execution/commitments/active",
            get(handlers::handle_commitments_active),
        )
        .route(
            "/api/execution/generate-recovery-plan",
            post(handlers::handle_generate_recovery_plan),
        )
        .route(
            "/api/execution/restore-workspace",
            post(handlers::handle_restore_workspace),
        )
        // ── Decision ──────────────────────────────────────────────────────
        .route(
            "/api/decision/simulate",
            post(handlers::handle_decision_simulate),
        )
        // ── Health ────────────────────────────────────────────────────────────
        .route("/api/health", get(handle_health))
        // ── Perception Ingest ─────────────────────────────────────────────────
        .route(
            "/api/perception/ingest",
            post(handlers::handle_perception_ingest),
        )
        // ── Telemetry Observability ───────────────────────────────────────────
        .route(
            "/api/telemetry/ack",
            post(handlers::handle_telemetry_ack),
        )
        .route(
            "/metrics",
            get(handlers::handle_metrics),
        )
        .with_state(state)
        .layer(cors)
}

/// Simple liveness probe endpoint.
async fn handle_health() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "ok": true,
        "service": "chronos-api-bridge",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::BridgeState;
    use axum::body::to_bytes;
    use axum::http::{Request, StatusCode};
    use chronos_memory_entity_resolution::EntityResolver;
    use chronos_memory_sessions::SessionEngine;
    use chronos_bus::MemoryEventBus;
    use chronos_registry::ServiceRegistry;
    use chronos_store_sqlite::SQLiteEventStore;
    use std::sync::Arc;
    use tempfile::NamedTempFile;
    use tokio::sync::RwLock;
    use tower::ServiceExt;

    fn make_state() -> BridgeState {
        let tmp = NamedTempFile::new().unwrap();
        // Keep the temp file alive for the duration of the test by leaking it
        let path = tmp.into_temp_path();
        let store = Arc::new(SQLiteEventStore::new(&path).unwrap());
        // Intentionally leak path so the db file lives for the test duration
        std::mem::forget(path);
        use chronos_reasoning_commitments::CommitmentState;
        use chronos_event_orchestrator::EventOrchestrator;
        BridgeState::new(
            store,
            Arc::new(ServiceRegistry::new()),
            Arc::new(RwLock::new(EntityResolver::new())),
            Arc::new(RwLock::new(None::<SessionEngine>)),
            Arc::new(RwLock::new(CommitmentState::new())),
            Arc::new(RwLock::new(EventOrchestrator::new())),
            Arc::new(MemoryEventBus::new(64)),
        )
    }

    #[tokio::test]
    async fn test_health_endpoint() {
        let state = make_state();
        let app = build_router(state);

        let req = Request::builder()
            .uri("/api/health")
            .body(axum::body::Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_events_stream_empty() {
        let state = make_state();
        let app = build_router(state);

        let req = Request::builder()
            .uri("/api/events/stream")
            .body(axum::body::Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["ok"], true);
        assert_eq!(json["data"]["total"], 0);
    }

    #[tokio::test]
    async fn test_state_endpoint_idle() {
        let state = make_state();
        let app = build_router(state);

        let req = Request::builder()
            .uri("/api/state")
            .body(axum::body::Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["ok"], true);
        // When idle, state_id must still be present
        assert!(json["data"]["state_id"].is_string());
    }

    #[tokio::test]
    async fn test_forecasts_endpoint_idle() {
        let state = make_state();
        let app = build_router(state);

        let req = Request::builder()
            .uri("/api/reasoning/forecasts")
            .body(axum::body::Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["ok"], true);
        // Idle response must report status
        assert_eq!(json["data"]["status"], "idle");
    }

    #[tokio::test]
    async fn test_diagnostics_endpoint_idle() {
        let state = make_state();
        let app = build_router(state);

        let req = Request::builder()
            .uri("/api/reasoning/diagnostics")
            .body(axum::body::Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_commitments_active_idle() {
        let state = make_state();
        let app = build_router(state);

        let req = Request::builder()
            .uri("/api/execution/commitments/active")
            .body(axum::body::Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["data"]["commitments"], serde_json::json!([]));
    }

    #[tokio::test]
    async fn test_decision_simulate_idle() {
        let state = make_state();
        let app = build_router(state);

        let req = Request::builder()
            .method("POST")
            .uri("/api/decision/simulate")
            .header("content-type", "application/json")
            .body(axum::body::Body::from(r#"{"override_urgency": null}"#))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["ok"], true);
        assert_eq!(json["data"]["status"], "idle");
    }
}
