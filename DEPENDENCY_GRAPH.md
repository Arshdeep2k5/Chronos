# System Dependency Graph: Chronos Pilot (v1.0)

This document visualizes the compile-time and runtime system boundaries. It highlights the downstream failure propagation paths to aid diagnostic sweeps during development.

## 1. Topological Graph

```text
       ┌──────────────────────┐
       │      SQLite (WAL)    │◄────────────────────────────────┐
       └──────────┬───────────┘                                 │
                  │                                             │
      ┌───────────┴───────────┬────────────────────────┐        │
      ▼                       ▼                        ▼        │
┌───────────┐           ┌───────────┐            ┌───────────┐  │
│ notify FS │           │  Browser  │            │ Workspace │  │
│  Watcher  │           │ Extension │            │ Snapshot  │  │
└─────┬─────┘           └─────┬─────┘            └─────┬─────┘  │
      │                       │                        │        │
      ▼                       ▼                        ▼        │
┌───────────┐           ┌───────────┐            ┌───────────┐  │
│ Ingestion │           │ Browser   │            │ Workspace │  │
│  Pipeline │           │ Sessions  │            │ Connector │  │
└─────┬─────┘           └─────┬─────┘            └─────┬─────┘  │
      │                       │                        │        │
      ▼                       ▼                        ▼        │
┌───────────┐           ┌───────────┐            ┌───────────┐  │
│Commitment │           │ Research  │            │ Context   │  │
│ Discovery │           │ Sessions  │            │ Restora-  │  │
└─────┬─────┘           └─────┬─────┘            │  tion     │  │
      │                       │                  └─────┬─────┘  │
      │                       ├────────────────────────┘        │
      │                       ▼                                 │
      │                 ┌───────────┐                           │
      │                 │    ARC    │                           │
      │                 └─────┬─────┘                           │
      │                       ▼                                 │
      ▼                 ┌───────────┐                           │
┌───────────┐           │ Research  │                           │
│ Failure   ├──────────►│   Brief   ├───────────────────────────┘
│ Forecast  │           └───────────┘
└─────┬─────┘
      ▼
┌───────────┐
│ Recovery  │
│ Planning  │
└───────────┘
```

## 2. Downstream Failure Propagation Paths

| Failure Point | Direct Downstream Impact | Mitigation Route |
| :--- | :--- | :--- |
| **Tauri loopback socket fails to bind** | Global termination. UI fails to load, background processes cannot communicate. | Fallback to Tauri fallback local named pipe IPC. |
| **Browser Extension crashes** | Research Session Reconstruction fails, ARC loses query context, and telemetry stops. | Fallback to system-level browser history file reading (`~/Library/.../History` on OS X). |
| **SQLite DB locked / SQLite-vec fails** | Ingestion pipeline stalls, system halts context updates. | Apply standard backoff loop (max 5 retries); drop back to in-memory fallback queues. |
| **Python Worker crashes** | CDE, local embeddings, and snapshot summary engines fail. | Restrain to simple, rule-based regex extraction in Rust; skip dynamic summaries. |
| **Workspace Connector (VSCode API) drops connection** | Active workspace cursor tracking and programmatic tab restoration fail. | Fallback to standard CLI file opening (`code <folder>`) which relies on VSCode default restoration state. |
