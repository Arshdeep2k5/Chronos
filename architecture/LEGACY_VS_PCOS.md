# LEGACY_VS_PCOS.md
*Authoritative Architectural Migration & Legacy-to-PCOS Transition Specification*

---

## 1. System Transition Matrix

The Chronos codebase is undergoing a migration from a monolithic Tauri framework to a modular, event-sourced PCOS architecture. The following matrix documents the migration status of every system component:

| Subsystem Component | Legacy Monolith (`src-tauri`) | Modern PCOS Daemon (`chronos-daemon`) | Transition Classification |
| :--- | :--- | :--- | :--- |
| **API Web Server** | Axum monolithic endpoints in `server.rs` | Modular endpoints in `chronos-api-bridge` | **Duplicate implementation** |
| **Database Persistence** | Relational relational tables in `db.rs` | Event-sourced ledger in `chronos-store-sqlite` | **Duplicate implementation** |
| **Event Routing** | None (direct DB queries) | Broadcast-based `MemoryEventBus` | **PCOS only** |
| **Window Focus Ingestion** | Win32 polling in `window_focus.rs` | Observer task in `chronos-adapter-window-focus` | **Duplicate implementation** |
| **Filesystem Ingestion** | Directory observer in `watcher.rs` | Observer task in `chronos-adapter-filewatcher` | **Duplicate implementation** |
| **Commitment Discoverer** | Heuristics engine in `che.rs` | Logical pipeline in `chronos-reasoning-commitments` | **Duplicate implementation** |
| **Workspace Restorer** | JSON restore helper in `server.rs` | Modular execution in `chronos-execution-runtime` | **Duplicate implementation** |
| **Browser Extension** | Intercepts tab focus via websocket | Intercepts tab focus via websocket | **Legacy only** (communicates via monolith ports) |
| **Python NLP Sidecars** | Run Monte Carlo forecasts in python | Mock fallback logic if python modules missing | **Legacy only** (managed by monolith subprocesses) |

---

## 2. Key Migration Pain Points

*   **Subprocess Spawning**: Spawning python workers is managed inside `src-tauri/src/main.rs`. The modern `chronos-daemon` does not launch or monitor these subprocesses natively; it assumes they are already running or maps calculations to fallback mock loops.
*   **Database Synchronization**: Legacy components write telemetry data directly to `chronos.db` and `chronos_telemetry.db`, while the modern PCOS daemon records events inside `chronos_events.db`. This dual-write behavior introduces SQLite concurrency locks and database write conflicts when both processes are active.

---

## 3. Recommended Consolidation Strategy

1.  **Consolidate WebSockets**: Redirect browser and IDE extension connections from ports 48120–48123 to the unified Axum bridge on port 7899.
2.  **Centralize Database Access**: Deprecate direct SQL writes in `src-tauri` and route all telemetry to `chronos-daemon` over REST endpoints, making `chronos_events.db` the single source of truth.

---
