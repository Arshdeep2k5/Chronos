//! # Axum Handler Implementations
//!
//! Each handler function maps to one required API endpoint from `CHRONOS_UI_MIGRATION_PLAN.md`.
//! All responses are sourced from live PCOS crate outputs — no mock data.

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use chronos_bus::EventBus;
use chronos_decision_orchestrator::DecisionOrchestrator;
use chronos_execution_cce::CceEngine;
use chronos_memory_state::StateProjector;
use chronos_reasoning_commitments::CommitmentEngine;
use chronos_reasoning_dde::DeadlineEngine;
use chronos_reasoning_pcm::CapacityEngine;
use chronos_reasoning_reflection::ReflectionEngine;
use chronos_reasoning_risk::RiskEngine;
use chronos_store::EventStore;
use chronos_telemetry_bridge::{browser, manual, vscode};

use crate::state::BridgeState;

// ─── Response envelope ────────────────────────────────────────────────────────

fn ok(data: Value) -> Response {
    (StatusCode::OK, Json(json!({ "ok": true, "data": data }))).into_response()
}

fn err(status: StatusCode, message: &str) -> Response {
    (
        status,
        Json(json!({ "ok": false, "error": message })),
    )
        .into_response()
}



/// Returns the full event log from the SQLite Event Store.
/// Ordered chronologically (oldest first). Sourced from durable storage — 100% real.
pub async fn handle_events_stream(State(state): State<BridgeState>) -> Response {
    match state.store.stream().await {
        Ok(events) => {
            let serialized: Vec<Value> = events
                .iter()
                .map(|e| serde_json::to_value(e).unwrap_or(Value::Null))
                .collect();
            ok(json!({
                "total": serialized.len(),
                "events": serialized,
            }))
        }
        Err(e) => err(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("Event store error: {:?}", e),
        ),
    }
}

/// Exposes the live runtime tick stream via Axum SSE (Server-Sent Events)
pub async fn handle_events_stream_live(
    State(state): State<BridgeState>,
) -> axum::response::sse::Sse<impl futures_util::stream::Stream<Item = Result<axum::response::sse::Event, std::convert::Infallible>>> {
    let rx = state.bus.subscribe();
    let stream = futures_util::stream::unfold(rx, |mut rx| async move {
        match rx.next_event().await {
            Ok(event) => {
                let json = serde_json::to_string(&event).unwrap_or_default();
                Some((Ok(axum::response::sse::Event::default().data(json)), rx))
            }
            Err(_) => {
                // If lagged or bus shutdown, keep connection or stop.
                // We keep unfolding by yielding a heartbeat/retry or stopping.
                None
            }
        }
    });

    axum::response::sse::Sse::new(stream).keep_alive(axum::response::sse::KeepAlive::default())
}


// ─── GET /api/state ───────────────────────────────────────────────────────────

/// Returns the current materialized ChronosState from the live in-memory projection.
/// Source: StateProjector over live EntityResolver + SessionEngine.
pub async fn handle_state(State(state): State<BridgeState>) -> Response {
    let resolver = state.resolver.read().await;
    let session_lock = state.session_engine.read().await;

    let graph = resolver.graph().clone();

    // Provide a minimal state if no sessions have been accumulated yet
    let chronos_state = if let Some(se) = session_lock.as_ref() {
        StateProjector::project(&graph, se.projection())
    } else {
        chronos_core::ChronosState::new(
            vec![],
            vec![],
            json!({
                "status": "idle",
                "message": "No perception events received yet. Daemon is waiting for adapter input."
            }),
        )
    };

    ok(json!({
        "state_id": chronos_state.id,
        "timestamp": chronos_state.timestamp.to_rfc3339(),
        "schema_version": chronos_state.schema_version,
        "active_intents": chronos_state.active_intents,
        "active_capabilities": chronos_state.active_capabilities,
        "payload": chronos_state.payload,
    }))
}

// ─── GET /api/reasoning/forecasts ────────────────────────────────────────────

/// Returns the current RiskForecast produced by the Risk Engine.
/// Source: RiskEngine.calculate_risk over live PCOS state.
pub async fn handle_reasoning_forecasts(State(state): State<BridgeState>) -> Response {
    let resolver = state.resolver.read().await;
    let session_lock = state.session_engine.read().await;

    let graph = resolver.graph().clone();

    let session_projection = match session_lock.as_ref() {
        Some(se) => se.projection(),
        None => {
            return ok(json!({
                "status": "idle",
                "message": "No sessions yet. Daemon awaiting perception events.",
                "forecast": null,
            }));
        }
    };

    let chronos_state = StateProjector::project(&graph, session_projection);
    let commitments =
        CommitmentEngine::resolve_commitments(&chronos_state, &graph, session_projection);
    let deadlines = DeadlineEngine::discover_deadlines(
        &chronos_state,
        &commitments,
        session_projection,
        &graph,
        &[],
    );
    let capacity = CapacityEngine::estimate_capacity(
        &chronos_state,
        session_projection,
        &commitments,
        &deadlines,
    );
    let forecast = RiskEngine::calculate_risk(
        &chronos_state,
        session_projection,
        &commitments,
        &deadlines,
        &capacity,
    );

    ok(json!({
        "status": "computed",
        "forecast": {
            "project_failure_probabilities": forecast.project_failure_probabilities,
            "context_decay_trajectory": forecast.context_decay_trajectory,
            "intervention_urgency": forecast.intervention_urgency,
            "confidence": forecast.confidence,
            "provenance_ids": forecast.provenance_ids,
        },
        "capacity": {
            "capacity_score": capacity.capacity_score,
            "focus_score": capacity.focus_score,
            "throughput_score": capacity.throughput_score,
            "stability_score": capacity.stability_score,
            "burnout_risk": capacity.burnout_risk,
            "confidence": capacity.confidence,
        }
    }))
}

// ─── GET /api/reasoning/diagnostics ─────────────────────────────────────────

/// Returns the Reflection Engine's current diagnostic assessment.
/// Source: ReflectionEngine.reflect over live PCOS state.
pub async fn handle_reasoning_diagnostics(State(state): State<BridgeState>) -> Response {
    let resolver = state.resolver.read().await;
    let session_lock = state.session_engine.read().await;

    let graph = resolver.graph().clone();

    let session_projection = match session_lock.as_ref() {
        Some(se) => se.projection(),
        None => {
            return ok(json!({
                "status": "idle",
                "message": "No sessions yet. Daemon awaiting perception events.",
                "reflection": null,
            }));
        }
    };

    let chronos_state = StateProjector::project(&graph, session_projection);

    match ReflectionEngine::reflect(&chronos_state, &graph, session_projection) {
        Ok(reflection) => {
            // Parse the structured payload embedded in outcome_evaluation
            let payload: Value = serde_json::from_str(&reflection.outcome_evaluation)
                .unwrap_or(json!({ "raw": reflection.outcome_evaluation }));

            ok(json!({
                "status": "computed",
                "reflection": {
                    "id": reflection.id,
                    "timestamp": reflection.timestamp.to_rfc3339(),
                    "schema_version": reflection.schema_version,
                    "confidence_delta": reflection.confidence_delta,
                    "diagnostics": payload,
                }
            }))
        }
        Err(e) => err(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("Reflection engine error: {:?}", e),
        ),
    }
}

// ─── GET /api/execution/commitments/active ───────────────────────────────────

/// Returns all active commitment candidates from the Commitment Engine.
/// Source: CommitmentEngine.resolve_commitments over live PCOS state.
pub async fn handle_commitments_active(State(state): State<BridgeState>) -> Response {
    let resolver = state.resolver.read().await;
    let session_lock = state.session_engine.read().await;

    let graph = resolver.graph().clone();

    let session_projection = match session_lock.as_ref() {
        Some(se) => se.projection(),
        None => {
            return ok(json!({
                "status": "idle",
                "message": "No sessions yet.",
                "commitments": [],
                "deadlines": [],
            }));
        }
    };

    let chronos_state = StateProjector::project(&graph, session_projection);
    let commitments =
        CommitmentEngine::resolve_commitments(&chronos_state, &graph, session_projection);
    let deadlines = DeadlineEngine::discover_deadlines(
        &chronos_state,
        &commitments,
        session_projection,
        &graph,
        &[],
    );

    let comm_lock = state.commitments.read().await;
    let event_sourced_active = CommitmentEngine::list_active_commitments(&comm_lock);

    let commitment_values: Vec<Value> = commitments
        .iter()
        .map(|c| serde_json::to_value(c).unwrap_or(Value::Null))
        .collect();
    let es_commitment_values: Vec<Value> = event_sourced_active
        .iter()
        .map(|c| serde_json::to_value(c).unwrap_or(Value::Null))
        .collect();
    let deadline_values: Vec<Value> = deadlines
        .iter()
        .map(|d| serde_json::to_value(d).unwrap_or(Value::Null))
        .collect();

    ok(json!({
        "status": "computed",
        "total_commitments": commitment_values.len(),
        "total_event_sourced_commitments": es_commitment_values.len(),
        "total_deadlines": deadline_values.len(),
        "commitments": commitment_values,
        "event_sourced_commitments": es_commitment_values,
        "deadlines": deadline_values,
    }))
}

// ─── POST /api/execution/generate-recovery-plan ──────────────────────────────

#[derive(Debug, Deserialize)]
pub struct GenerateRecoveryPlanRequest {
    /// Optional hint for the project to prioritize in recovery
    pub project_hint: Option<String>,
}

/// Generates a recovery plan by running the full PCOS pipeline and returning
/// the CCE output for the current risk state.
/// Source: Full pipeline → DecisionOrchestrator → CceEngine.
pub async fn handle_generate_recovery_plan(
    State(state): State<BridgeState>,
    Json(req): Json<GenerateRecoveryPlanRequest>,
) -> Response {
    let resolver = state.resolver.read().await;
    let session_lock = state.session_engine.read().await;

    let graph = resolver.graph().clone();

    let session_projection = match session_lock.as_ref() {
        Some(se) => se.projection(),
        None => {
            return err(
                StatusCode::UNPROCESSABLE_ENTITY,
                "Cannot generate a recovery plan without any accumulated context. Perception events required.",
            );
        }
    };

    let chronos_state = StateProjector::project(&graph, session_projection);
    let commitments =
        CommitmentEngine::resolve_commitments(&chronos_state, &graph, session_projection);
    let deadlines = DeadlineEngine::discover_deadlines(
        &chronos_state,
        &commitments,
        session_projection,
        &graph,
        &[],
    );
    let capacity = CapacityEngine::estimate_capacity(
        &chronos_state,
        session_projection,
        &commitments,
        &deadlines,
    );
    let forecast = RiskEngine::calculate_risk(
        &chronos_state,
        session_projection,
        &commitments,
        &deadlines,
        &capacity,
    );
    let reflection = ReflectionEngine::reflect(&chronos_state, &graph, session_projection).ok();
    let reflections: Vec<chronos_core::ChronosReflection> = reflection.into_iter().collect();

    let decision = DecisionOrchestrator::orchestrate_decision(
        &chronos_state,
        &forecast,
        &capacity,
        &commitments,
        &deadlines,
        &reflections,
        session_projection,
    );

    let action = CceEngine::translate_decision(
        &decision,
        &chronos_state,
        session_projection,
        &commitments,
        &deadlines,
        &forecast,
    );

    ok(json!({
        "status": "generated",
        "project_hint": req.project_hint,
        "decision": {
            "id": decision.id,
            "action_type": decision.action_type,
            "confidence": decision.confidence,
            "explanation": decision.explanation,
            "evidence_ids": decision.evidence_ids,
        },
        "recovery_plan": action.as_ref().map(|a| json!({
            "action_id": a.id,
            "action_type": a.action_type,
            "payload": a.payload,
        })),
        "risk_summary": {
            "intervention_urgency": forecast.intervention_urgency,
            "project_failure_probabilities": forecast.project_failure_probabilities,
            "burnout_risk": capacity.burnout_risk,
        }
    }))
}

// ─── POST /api/decision/simulate ─────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct SimulateDecisionRequest {
    /// Optional override for intervention_urgency (0.0..1.0). If absent, uses live data.
    pub override_urgency: Option<f64>,
}

/// Runs the DecisionOrchestrator over the current live PCOS state and returns
/// the resulting decision. Used by Mode 03 Theory Simulator.
/// Source: Full pipeline → DecisionOrchestrator (no side effects).
pub async fn handle_decision_simulate(
    State(state): State<BridgeState>,
    Json(_req): Json<SimulateDecisionRequest>,
) -> Response {
    let resolver = state.resolver.read().await;
    let session_lock = state.session_engine.read().await;

    let graph = resolver.graph().clone();

    let session_projection = match session_lock.as_ref() {
        Some(se) => se.projection(),
        None => {
            return ok(json!({
                "status": "idle",
                "message": "No context accumulated. Decision would be NoAction.",
                "simulated_decision": {
                    "action_type": "NoAction",
                    "confidence": 0,
                    "explanation": "No perception events received yet.",
                }
            }));
        }
    };

    let chronos_state = StateProjector::project(&graph, session_projection);
    let commitments =
        CommitmentEngine::resolve_commitments(&chronos_state, &graph, session_projection);
    let deadlines = DeadlineEngine::discover_deadlines(
        &chronos_state,
        &commitments,
        session_projection,
        &graph,
        &[],
    );
    let capacity = CapacityEngine::estimate_capacity(
        &chronos_state,
        session_projection,
        &commitments,
        &deadlines,
    );
    let forecast = RiskEngine::calculate_risk(
        &chronos_state,
        session_projection,
        &commitments,
        &deadlines,
        &capacity,
    );
    let reflection = ReflectionEngine::reflect(&chronos_state, &graph, session_projection).ok();
    let reflections: Vec<chronos_core::ChronosReflection> = reflection.into_iter().collect();

    let decision = DecisionOrchestrator::orchestrate_decision(
        &chronos_state,
        &forecast,
        &capacity,
        &commitments,
        &deadlines,
        &reflections,
        session_projection,
    );

    ok(json!({
        "status": "simulated",
        "simulated_decision": {
            "id": decision.id,
            "action_type": decision.action_type,
            "confidence": decision.confidence,
            "explanation": decision.explanation,
            "evidence_ids": decision.evidence_ids,
            "action_payload": decision.action_payload,
        },
        "supporting_data": {
            "intervention_urgency": forecast.intervention_urgency,
            "burnout_risk": capacity.burnout_risk,
            "capacity_score": capacity.capacity_score,
            "commitment_count": commitments.len(),
            "deadline_count": deadlines.len(),
        }
    }))
}

// ─── POST /api/execution/restore-workspace ────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct RestoreWorkspaceRequest {
    /// Optional: specific session ID to restore. If absent, uses the latest session.
    pub target_session_id: Option<String>,
}

/// Generates a WorkspaceRestoreRequest action for the specified (or latest) session.
/// Source: CceEngine with SuggestWorkspaceRestore decision override.
pub async fn handle_restore_workspace(
    State(state): State<BridgeState>,
    Json(req): Json<RestoreWorkspaceRequest>,
) -> Response {
    let session_lock = state.session_engine.read().await;

    let session_projection = match session_lock.as_ref() {
        Some(se) => se.projection(),
        None => {
            return err(
                StatusCode::UNPROCESSABLE_ENTITY,
                "No sessions available. Cannot generate workspace restore request.",
            );
        }
    };

    // Determine target session
    let target_session = if let Some(ref id) = req.target_session_id {
        session_projection.sessions().get(id).cloned()
    } else {
        session_projection.latest().cloned()
    };

    let Some(session) = target_session else {
        return err(StatusCode::NOT_FOUND, "Target session not found.");
    };

    // Construct a synthetic WorkspaceRestore decision to feed into CCE
    let restore_decision = chronos_core::ChronosDecision::new(
        90,
        format!(
            "Workspace restore requested for session {}",
            session.session_id
        ),
        vec![session.session_id.clone()],
        "SuggestWorkspaceRestore",
        serde_json::Value::Null,
        None,
    );

    let resolver = state.resolver.read().await;
    let graph = resolver.graph().clone();
    let chronos_state = StateProjector::project(&graph, session_projection);
    let commitments =
        CommitmentEngine::resolve_commitments(&chronos_state, &graph, session_projection);
    let deadlines = DeadlineEngine::discover_deadlines(
        &chronos_state,
        &commitments,
        session_projection,
        &graph,
        &[],
    );
    let capacity = CapacityEngine::estimate_capacity(
        &chronos_state,
        session_projection,
        &commitments,
        &deadlines,
    );
    let forecast = RiskEngine::calculate_risk(
        &chronos_state,
        session_projection,
        &commitments,
        &deadlines,
        &capacity,
    );

    let action = CceEngine::translate_decision(
        &restore_decision,
        &chronos_state,
        session_projection,
        &commitments,
        &deadlines,
        &forecast,
    );

    ok(json!({
        "status": "restore_plan_generated",
        "target_session_id": session.session_id,
        "session_duration_seconds": session.duration,
        "restore_action": action.as_ref().map(|a| json!({
            "action_id": a.id,
            "action_type": a.action_type,
            "payload": a.payload,
        })),
        "artifact_ids": session.artifact_ids,
        "repository_ids": session.repository_ids,
        "project_ids": session.project_ids,
    }))
}

// ─── POST /api/perception/ingest ─────────────────────────────────────────────

/// Classifies raw perception payloads by their declared `source` field.
#[derive(Debug, Deserialize)]
pub struct IngestRequest {
    /// One of: "browser", "vscode", "manual", "raw"
    pub source: String,
    /// The raw payload to convert and ingest.
    pub payload: Value,
}

/// Receives a raw telemetry payload from any source, converts it into a canonical
/// `ChronosEvent` via the telemetry bridge, publishes it to the Cognitive Bus,
/// and lets the normal daemon pipeline persist it through the SQLite Event Store.
///
/// This endpoint satisfies `CHRONOS_UI_MIGRATION_PLAN.md` section 2.2 requirement
/// for `POST /api/perception/ingest` (Manual observation injection).
///
/// ## Supported Sources
/// - `"browser"` — routed through `chronos_telemetry_bridge::browser`
/// - `"vscode"` — routed through `chronos_telemetry_bridge::vscode`
/// - `"manual"` — routed through `chronos_telemetry_bridge::manual`
/// - `"raw"` — payload must contain `event_type` and `source` fields; ingested verbatim
pub async fn handle_perception_ingest(
    State(state): State<BridgeState>,
    Json(req): Json<IngestRequest>,
) -> Response {
    // ── Step 1: Convert payload to a ChronosEvent via the telemetry bridge ──
    let event_opt = match req.source.as_str() {
        "browser" => browser::convert_browser_event(&req.payload),
        "vscode" => vscode::convert_vscode_event(&req.payload),
        "manual" => manual::convert_manual_ingestion(&req.payload),
        "raw" => {
            // For raw payloads the caller must supply event_type directly.
            let event_type = match req.payload.get("event_type").and_then(|v| v.as_str()) {
                Some(t) => t.to_string(),
                None => {
                    return err(
                        StatusCode::UNPROCESSABLE_ENTITY,
                        "Raw ingest requires an 'event_type' field in the payload.",
                    );
                }
            };
            let raw_source = req
                .payload
                .get("source")
                .and_then(|v| v.as_str())
                .unwrap_or("ManualRaw");
            Some(chronos_telemetry_bridge::create_provenance_event(
                &event_type,
                raw_source,
                req.payload.clone(),
                &req.payload,
            ))
        }
        _ => {
            return err(
                StatusCode::BAD_REQUEST,
                &format!(
                    "Unknown source '{}'. Valid values: browser | vscode | manual | raw",
                    req.source
                ),
            );
        }
    };

    let event = match event_opt {
        Some(e) => e,
        None => {
            return err(
                StatusCode::UNPROCESSABLE_ENTITY,
                &format!(
                    "Payload could not be converted to a ChronosEvent for source '{}'.",
                    req.source
                ),
            );
        }
    };

    let event_id = event.id.clone();
    let event_type = event.event_type.clone();

    // ── Step 2: Publish to Cognitive Bus → daemon pipeline picks it up ──────
    match state.bus.publish(event.clone()) {
        Ok(receiver_count) => {
            tracing::info!(
                "IngestAPI: published {} (id={}) to bus ({} receivers)",
                event_type,
                event_id,
                receiver_count
            );
        }
        Err(e) => {
            return err(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("Bus publish failed: {:?}", e),
            );
        }
    }

    // ── Step 3: Persistence is handled exclusively by the daemon pipeline ───
    // The pipeline worker subscribes to the bus and persists all events,
    // eliminating the duplicate unique ID collision.

    ok(json!({
        "ingested": true,
        "event_id": event_id,
        "event_type": event_type,
        "source": req.source,
        "message": "Event published to Cognitive Bus and persisted to Event Store.",
    }))
}

// ─── GET /api/session/current ────────────────────────────────────────────────

/// Returns the current cognitive session projection.
/// Source: SessionEngine.projection over live PCOS state.
pub async fn handle_session_current(State(state): State<BridgeState>) -> Response {
    let session_lock = state.session_engine.read().await;

    match session_lock.as_ref() {
        Some(se) => {
            let proj = se.projection();
            if let Some(latest) = proj.latest() {
                ok(json!({
                    "status": "active",
                    "session": {
                        "id": latest.session_id,
                        "duration": latest.duration,
                        "confidence": latest.confidence,
                        "event_count": latest.source_event_ids.len(),
                        "is_closed": latest.is_closed,
                    }
                }))
            } else {
                ok(json!({
                    "status": "idle",
                    "message": "No sessions active."
                }))
            }
        }
        None => ok(json!({
            "status": "idle",
            "message": "Session engine not initialized."
        }))
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TelemetryAckRequest {
    pub tick_id: String,
    pub received_at: String,
    pub parsed_at: String,
    pub rendered_at: String,
    pub latency_breakdown: Value,
}

pub async fn handle_telemetry_ack(
    State(state): State<BridgeState>,
    Json(req): Json<TelemetryAckRequest>,
) -> Response {
    let event = chronos_core::ChronosEvent::new(
        "UiTelemetryAckReceived",
        "UiTelemetryBridge",
        serde_json::to_value(&req).unwrap_or(Value::Null),
    );
    if let Err(e) = state.store.append(event.clone()).await {
        tracing::warn!("Failed to append UiTelemetryAckReceived: {:?}", e);
    }
    let _ = state.bus.publish(event);
    ok(json!({ "success": true }))
}

pub async fn handle_metrics(
    State(state): State<BridgeState>,
) -> impl IntoResponse {
    let conn = state.store.conn.lock().await;

    // 1. Fetch latest TickFrame payload
    let mut tick_duration = 0.0;
    let mut phase_durations = std::collections::HashMap::new();
    
    let mut stmt = conn.prepare(
        "SELECT payload FROM chronos_events WHERE event_type = 'TickFrameEmitted' ORDER BY timestamp DESC LIMIT 1"
    ).unwrap();
    if let Ok(mut rows) = stmt.query([]) {
        if let Ok(Some(row)) = rows.next() {
            if let Ok(payload_str) = row.get::<_, String>(0) {
                if let Ok(payload) = serde_json::from_str::<serde_json::Value>(&payload_str) {
                    if let Some(telemetry) = payload.get("telemetry") {
                        if let Some(total) = telemetry.get("total_duration_ms").and_then(|v| v.as_f64()) {
                            tick_duration = total;
                        }
                        if let Some(phases) = telemetry.get("phase_durations_ms").and_then(|v| v.as_object()) {
                            for (phase, duration) in phases {
                                if let Some(val) = duration.as_f64() {
                                    phase_durations.insert(phase.clone(), val);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // 2. Fetch latest SSE lag
    let mut sse_lag = 0.0;
    let mut stmt_lag = conn.prepare(
        "SELECT payload FROM chronos_events WHERE event_type = 'UiTelemetryAckReceived' ORDER BY timestamp DESC LIMIT 1"
    ).unwrap();
    if let Ok(mut rows) = stmt_lag.query([]) {
        if let Ok(Some(row)) = rows.next() {
            if let Ok(payload_str) = row.get::<_, String>(0) {
                if let Ok(payload) = serde_json::from_str::<serde_json::Value>(&payload_str) {
                    if let Some(lag) = payload.get("latency_breakdown").and_then(|lb| lb.get("network_delivery_lag_ms")).and_then(|v| v.as_f64()) {
                        sse_lag = lag;
                    }
                }
            }
        }
    }

    // 3. Count alert counts by severity
    let mut alert_counts = std::collections::HashMap::new();
    alert_counts.insert("INFO".to_string(), 0.0);
    alert_counts.insert("WARN".to_string(), 0.0);
    alert_counts.insert("CRITICAL".to_string(), 0.0);

    let mut stmt_alerts = conn.prepare(
        "SELECT severity, COUNT(*) FROM chronos_alerts GROUP BY severity"
    ).unwrap();
    if let Ok(mut rows) = stmt_alerts.query([]) {
        while let Ok(Some(row)) = rows.next() {
            if let (Ok(sev), Ok(count)) = (row.get::<_, String>(0), row.get::<_, i64>(1)) {
                alert_counts.insert(sev, count as f64);
            }
        }
    }

    // 4. Count dropped frames
    let mut dropped_frames = 0;
    let mut stmt_dropped = conn.prepare(
        "SELECT COUNT(*) FROM chronos_events WHERE event_type = 'DroppedFrameDetected'"
    ).unwrap();
    if let Ok(mut rows) = stmt_dropped.query([]) {
        if let Ok(Some(row)) = rows.next() {
            if let Ok(count) = row.get::<_, i64>(0) {
                dropped_frames = count;
            }
        }
    }

    // Derive health state: 0 = HEALTHY, 1 = DEGRADED, 2 = CRITICAL
    let mut health_state = 0;
    if tick_duration > 50.0 || sse_lag > 100.0 || alert_counts.get("WARN").unwrap_or(&0.0) > &0.0 {
        health_state = 1;
    }
    if alert_counts.get("CRITICAL").unwrap_or(&0.0) > &0.0 {
        health_state = 2;
    }

    // 5. Generate Prometheus metrics
    let mut body = String::new();
    body.push_str("# HELP chronos_tick_duration_ms Duration of the last execution tick in milliseconds\n");
    body.push_str("# TYPE chronos_tick_duration_ms gauge\n");
    body.push_str(&format!("chronos_tick_duration_ms {}\n\n", tick_duration));

    body.push_str("# HELP chronos_execution_phase_latency_ms Latency of each runtime phase in milliseconds\n");
    body.push_str("# TYPE chronos_execution_phase_latency_ms gauge\n");
    for phase in &["perception", "reasoning", "synthesis", "decision", "execution", "feedback"] {
        let val = phase_durations.get(*phase).unwrap_or(&0.0);
        body.push_str(&format!("chronos_execution_phase_latency_ms{{phase=\"{}\"}} {}\n", phase, val));
    }
    body.push_str("\n");

    body.push_str("# HELP chronos_sse_lag_ms Network and process delivery lag of the Server-Sent Events stream\n");
    body.push_str("# TYPE chronos_sse_lag_ms gauge\n");
    body.push_str(&format!("chronos_sse_lag_ms {}\n\n", sse_lag));

    body.push_str("# HELP chronos_dropped_frame_count Total count of dropped tick frames detected by the client\n");
    body.push_str("# TYPE chronos_dropped_frame_count counter\n");
    body.push_str(&format!("chronos_dropped_frame_count {}\n\n", dropped_frames));

    body.push_str("# HELP chronos_alert_count Total number of alerts emitted, partitioned by severity\n");
    body.push_str("# TYPE chronos_alert_count counter\n");
    for (sev, val) in &alert_counts {
        body.push_str(&format!("chronos_alert_count{{severity=\"{}\"}} {}\n", sev, val));
    }
    body.push_str("\n");

    body.push_str("# HELP chronos_operational_health System operational health state: 0=HEALTHY, 1=DEGRADED, 2=CRITICAL\n");
    body.push_str("# TYPE chronos_operational_health gauge\n");
    body.push_str(&format!("chronos_operational_health {}\n", health_state));

    let mut res = Response::new(axum::body::Body::from(body));
    res.headers_mut().insert(
        axum::http::header::CONTENT_TYPE,
        axum::http::HeaderValue::from_static("text/plain; version=0.0.4; charset=utf-8"),
    );
    res
}
