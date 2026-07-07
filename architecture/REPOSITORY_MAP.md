# REPOSITORY_MAP.md
*Authoritative Workspace Directory & Crate Map Specification*

---

## 1. Directory Tree & Key Folders

This directory map outlines the structure of the Chronos repository to help developers locate files quickly:

```
D:\Chronos_Hackathon
 ├── .github/workflows/          # CI/CD pipelines (SAST, test suites)
 ├── architecture/               # Unified architectural documentation
 ├── chronos-pilot/              # Active Single Page React Dashboard
 │    ├── src/
 │    │    ├── cognitive/        # Frontend Cognitive Semantic Layer (CSL)
 │    │    ├── cognitive-visualization/ # Physics-based Graph Visualizer
 │    │    └── components/       # Telemetry console widgets
 │    └── Dockerfile             # Production Node container builder
 ├── python-worker/              # Sidecars (NLP, forecasts, web crawlers)
 ├── extensions/                 # IDE and Browser telemetry collectors
 │    ├── vscode-connector/      # VSCode editor telemetry extension
 │    └── browser-extension/     # MV3 browser tab observer
 ├── src-tauri/                  # Legacy Monolith Backend
 │    └── src/
 │         ├── server.rs         # Ephemeral HTTP endpoints
 │         └── db.rs             # SQLite schemas configuration
 ├── chronos-daemon/             # PCOS unified Daemon Process (main.rs)
 ├── Dockerfile.daemon           # Docker container builder for the daemon
 └── Cargo.toml                  # Rust dependency configuration
```

---

## 2. PCOS Layer Crate Map

Crates are organized by their corresponding PCOS layer:

*   **Layer 0 (Infrastructure)**:
    *   `chronos-core/`: Schema definitions for the Chronos Object Model (COM).
    *   `chronos-bus/`: Asynchronous memory bus broadcast broker.
    *   `chronos-store/`: EventStore interfaces.
    *   `chronos-store-sqlite/`: SQLite event store implementation.
    *   `chronos-registry/`: Thread-safe service directory.
    *   `chronos-container/`: Lightweight IoC DI container.
    *   `chronos-config/`: Configuration settings loader.
    *   `chronos-logging/`: Structured logging wrapper around tracing.
*   **Layer 1 (Perception)**:
    *   `chronos-adapter-filewatcher/`: Filesystem observer.
    *   `chronos-adapter-git/`: Git log observer.
    *   `chronos-adapter-window-focus/`: Win32 window focus observer.
    *   `chronos-adapter-clipboard/`: Clipboard copy observer.
*   **Layer 2 (Memory)**:
    *   `chronos-memory-entity-resolution/`: Matches raw events to semantic graphs.
    *   `chronos-memory-sessions/`: Groups events into continuous sessions.
    *   `chronos-memory-state/`: Materializes global state snapshots.
*   **Layer 3 (Reasoning)**:
    *   `chronos-reasoning-reflection/`: Interprets project drifts and stalled tasks.
    *   `chronos-reasoning-commitments/`: Discovers explicit/implicit obligations.
    *   `chronos-reasoning-dde/`: Deadline Discovery Engine.
    *   `chronos-reasoning-pcm/`: Personal Capacity Model.
    *   `chronos-reasoning-risk/`: Logistic risk forecasting engine.
*   **Layer 4 (Decision)**:
    *   `chronos-decision-orchestrator/`: Unified Decision Engine.
*   **Layer 5 (Execution)**:
    *   `chronos-execution-cce/`: Context Continuation Engine.
    *   `chronos-execution-runtime/`: System action dispatchers.
*   **Layer 6 (Interaction)**:
    *   `chronos-api-bridge/`: REST routes and SSE stream endpoints.

---
