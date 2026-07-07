# CHRONOS_DAEMON_SPEC.md
## Chronos Daemon — Specification
*Version 1.0 | PCOS Layer 0 Runtime*

---

## 1. Purpose

`chronos-daemon` is the production runtime entry point for the entire PCOS ecosystem.
It is responsible for:
- Initializing all PCOS infrastructure services
- Registering all engines, adapters, and stores in the ServiceRegistry
- Replaying persisted events to rebuild warm in-memory PCOS state on every startup
- Running the end-to-end event processing pipeline as a background worker
- Routing outputs (forecasts, decisions, actions) back to the Event Store and Bus

The daemon does NOT expose any HTTP endpoints. It consumes the Cognitive Bus and writes to the SQLite Event Store. The `chronos-api-bridge` serves HTTP endpoints reading from the same store.

---

## 2. Crate Structure

```
chronos-daemon/
├── Cargo.toml         — all PCOS crate dependencies
└── src/
    └── main.rs        — daemon binary entry point
```

---

## 3. Boot Sequence

| Step | Action | Failure Behavior |
|------|--------|-----------------|
| 1 | Initialize tracing/logging | Panic |
| 2 | Resolve `CHRONOS_DATA_DIR` (env var or OS data dir) | Panic |
| 3 | Create data directory if absent | Panic |
| 4 | Open SQLite Event Store at `{data_dir}/chronos_events.db` | Panic |
| 5 | Initialize Cognitive Bus (broadcast channel, capacity 4096) | Panic |
| 6 | Initialize Service Registry | — |
| 7 | Register all 12 PCOS services with Running/Healthy status | Warn per failure |
| 8 | Stream all persisted events from store → replay through Memory layer | Warn per error |
| 9 | Spawn pipeline worker task (subscribes to Bus) | — |
| 10 | Block on `Ctrl+C` / SIGTERM | — |
| 11 | Abort pipeline worker | — |
| 12 | Log clean shutdown | — |

---

## 4. Registered Services (12)

| Service ID | Name | Type | Capabilities |
|------------|------|------|--------------|
| `svc-event-store-sqlite` | SQLite Event Store | Storage | PersistEvents |
| `svc-cognitive-bus` | Cognitive Bus | Transport | RouteEvents |
| `svc-entity-resolver` | Entity Resolver | Engine | ResolveEntities |
| `svc-session-engine` | Session Engine | Engine | ManageSessions |
| `svc-state-projector` | State Projector | Engine | ProjectState |
| `svc-reflection-engine` | Reflection Engine | Engine | GenerateReflections |
| `svc-commitment-engine` | Commitment Engine | Engine | DiscoverCommitments |
| `svc-dde` | Deadline Discovery Engine | Engine | DiscoverDeadlines |
| `svc-pcm` | Personal Capacity Model | Engine | EstimateCapacity |
| `svc-risk-engine` | Risk Forecast Engine | Engine | ForecastRisk |
| `svc-decision-orchestrator` | Decision Orchestrator | Engine | OrchestrateDecisions |
| `svc-cce` | Context Continuation Engine | Engine | TranslateDecisions |

---

## 5. Event Pipeline

```
Cognitive Bus (subscriber)
    ↓ event received
Persist raw event → SQLite Event Store
    ↓ skip pipeline-output events (loop prevention)
Layer 2: Memory
    EntityResolver.process_event(event)
    SessionEngine.process_event(event)
    StateProjector.project(graph, sessions)
    ↓
Layer 3: Reasoning
    CommitmentEngine.resolve_commitments(state, graph, sessions)
    DeadlineEngine.discover_deadlines(state, commitments, sessions, graph)
    CapacityEngine.estimate_capacity(state, sessions, commitments, deadlines)
    RiskEngine.calculate_risk(state, sessions, commitments, deadlines, capacity)
    ReflectionEngine.reflect(state, graph, sessions)
    ↓ persist + publish RiskForecastResolved
Layer 4: Decision
    DecisionOrchestrator.orchestrate_decision(state, forecast, capacity, ...)
    ↓ persist + publish DecisionResolved
Layer 5: Execution
    CceEngine.translate_decision(decision, state, sessions, ...)
    ↓ persist + publish action event (if applicable)
```

### Pipeline Loop Prevention
Events with the following `event_type` values are skipped by the pipeline subscriber to prevent feedback loops:
- `DecisionResolved`
- `RiskForecastResolved`
- `ActionStarted`
- `ActionCompleted`
- `ActionFailed`
- `ContinuationPlanResolved`

---

## 6. Configuration

| Environment Variable | Default | Description |
|---------------------|---------|-------------|
| `CHRONOS_DATA_DIR` | `{OS_DATA_DIR}/chronos` | Path to data directory |
| `RUST_LOG` | `info` | Log level filter |

---

## 7. Replay Safety

On startup, the daemon replays all persisted events through the Memory layer engines (EntityResolver and SessionEngine) to rebuild warm in-memory state. This ensures:
- The state after restart is identical to the state before shutdown
- No data loss across daemon restarts
- Consistent behavior for the API bridge

Replay excludes pipeline-output events (same set as loop-prevention filter) to avoid re-triggering decisions on historical data.

---

## 8. Tests

| Test | Description |
|------|-------------|
| `test_daemon_service_registration` | Verifies all 12 services register as Running/Healthy |
| `test_daemon_store_initialization` | Verifies fresh SQLite store has 0 events |
| `test_replay_determinism` | Verifies identical event replay yields identical entity/session counts |
| `test_end_to_end_pipeline_cycle` | Injects a real perception event, verifies DecisionResolved + RiskForecastResolved appear in store |
