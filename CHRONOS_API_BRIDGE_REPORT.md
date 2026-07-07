# CHRONOS_API_BRIDGE_REPORT.md
## Chronos API Bridge — Implementation Report
*Version 1.0 | PCOS Layer API*

---

## 1. Purpose

`chronos-api-bridge` implements the HTTP API layer specified in `CHRONOS_UI_MIGRATION_PLAN.md`.
It is a standalone Axum binary that reads from the same SQLite Event Store as `chronos-daemon`
and serves real-time PCOS outputs to the Chronos UI.

**Zero mock data. Zero placeholder logic. All responses sourced from live PCOS crate outputs.**

---

## 2. Architecture

```
UI (React/Tauri)
    ↕ HTTP (localhost:7899)
chronos-api-bridge (Axum)
    ↔ SQLiteEventStore (shared DB file)
    ↔ In-memory PCOS state (warmed from event replay on startup)
    ← ChronosCore crates (RiskEngine, DecisionOrchestrator, CceEngine, etc.)
```

The bridge does **not** subscribe to the Cognitive Bus. It builds its in-memory PCOS state
by replaying persisted events from the SQLite Event Store on startup, then re-computes
all PCOS outputs on every API request from the live in-memory state.

---

## 3. Implemented Endpoints

| Method | Path | Source | Status |
|--------|------|--------|--------|
| GET | `/api/health` | Static | ✅ |
| GET | `/api/events/stream` | SQLite EventStore | ✅ |
| GET | `/api/state` | StateProjector | ✅ |
| GET | `/api/reasoning/forecasts` | RiskEngine + CapacityEngine | ✅ |
| GET | `/api/reasoning/diagnostics` | ReflectionEngine | ✅ |
| GET | `/api/execution/commitments/active` | CommitmentEngine + DeadlineEngine | ✅ |
| POST | `/api/execution/generate-recovery-plan` | Full pipeline → CceEngine | ✅ |
| POST | `/api/decision/simulate` | Full pipeline → DecisionOrchestrator | ✅ |
| POST | `/api/execution/restore-workspace` | CceEngine (WorkspaceRestoreRequest) | ✅ |

---

## 4. Response Schema

All responses follow the unified envelope:

```json
{ "ok": true,  "data": { ... } }
{ "ok": false, "error": "message" }
```

### GET /api/state

```json
{
  "ok": true,
  "data": {
    "state_id": "uuid",
    "timestamp": "ISO-8601",
    "schema_version": "1.0",
    "active_intents": ["session-id"],
    "active_capabilities": [],
    "payload": { ... ProjectedStatePayload ... }
  }
}
```

### GET /api/reasoning/forecasts

```json
{
  "ok": true,
  "data": {
    "status": "computed",
    "forecast": {
      "project_failure_probabilities": { "entity-id": 0.42 },
      "context_decay_trajectory": { "entity-id": 0.15 },
      "intervention_urgency": 0.42,
      "confidence": 0.90,
      "provenance_ids": ["session-id", "deadline-id"]
    },
    "capacity": {
      "capacity_score": 0.80,
      "focus_score": 0.75,
      "throughput_score": 0.85,
      "stability_score": 0.70,
      "burnout_risk": 0.20,
      "confidence": 0.85
    }
  }
}
```

### POST /api/decision/simulate

```json
{
  "ok": true,
  "data": {
    "status": "simulated",
    "simulated_decision": {
      "id": "uuid",
      "action_type": "SuggestWorkspaceRestore",
      "confidence": 87,
      "explanation": "Inactivity timeout detected...",
      "evidence_ids": ["session-id"],
      "action_payload": { ... }
    },
    "supporting_data": {
      "intervention_urgency": 0.55,
      "burnout_risk": 0.20,
      "capacity_score": 0.80,
      "commitment_count": 2,
      "deadline_count": 1
    }
  }
}
```

---

## 5. Idle State Handling

When no perception events have been received yet (cold start), all endpoints return
graceful idle responses rather than errors:

| Endpoint | Idle Response |
|----------|--------------|
| `/api/state` | State with `payload.status = "idle"` |
| `/api/reasoning/forecasts` | `{ "status": "idle", "forecast": null }` |
| `/api/reasoning/diagnostics` | `{ "status": "idle", "reflection": null }` |
| `/api/execution/commitments/active` | `{ "commitments": [], "deadlines": [] }` |
| `/api/decision/simulate` | `{ "status": "idle", "simulated_decision": { "action_type": "NoAction" } }` |
| `/api/execution/generate-recovery-plan` | HTTP 422 with explanatory message |
| `/api/execution/restore-workspace` | HTTP 422 with explanatory message |

---

## 6. Configuration

| Environment Variable | Default | Description |
|---------------------|---------|-------------|
| `CHRONOS_DATA_DIR` | `{OS_DATA_DIR}/chronos` | Path to data directory (same as daemon) |
| `CHRONOS_API_PORT` | `7899` | HTTP port |
| `RUST_LOG` | `info` | Log level filter |

---

## 7. Test Results

```
running 7 tests
test router::tests::test_health_endpoint              ... ok
test router::tests::test_events_stream_empty          ... ok
test router::tests::test_decision_simulate_idle       ... ok
test router::tests::test_diagnostics_endpoint_idle    ... ok
test router::tests::test_commitments_active_idle      ... ok
test router::tests::test_state_endpoint_idle          ... ok
test router::tests::test_forecasts_endpoint_idle      ... ok

test result: ok. 7 passed; 0 failed; 0 ignored
```

All 7 endpoint tests pass. Build produces zero warnings.

---

## 8. Crate Structure

```
chronos-api-bridge/
├── Cargo.toml
└── src/
    ├── lib.rs          — module declarations + documentation
    ├── main.rs         — Axum server entry point
    ├── state.rs        — BridgeState shared Axum extension
    ├── router.rs       — route registration, CORS, router tests
    └── handlers.rs     — 8 handler functions + response envelope
```
