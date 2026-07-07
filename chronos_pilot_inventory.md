# Chronos Pilot UI & Backend Architecture Inventory Map

This file provides a complete file-by-file inventory and architectural mapping of the `chronos-pilot` frontend system and its connected Rust backend interfaces.

---

## ─── SYSTEM DATA & TELEMETRY FLOW ───

The diagram below outlines how the real-time event pipeline moves from cognitive execution to the UI projection layer and triggers backends for operational telemetry scraping:

```mermaid
graph TD
    %% Backend pipeline
    subgraph Rust Daemon Loop
        Daemon[chronos-daemon / main.rs] -->|Tick execution| Loop[ContinuousRuntimeLoopEngine]
        Loop -->|TickFrameEmitted event| Store[SQLiteEventStore / db_path]
        Loop -->|TickPerformanceWarning| Bus[MemoryEventBus]
        Store -->|Persists raw records| Sqlite[(chronos_events.db)]
    end

    %% API Bridge
    subgraph API Bridge Layer (chronos-api-bridge)
        Router[router.rs] -->|SSE Endpoint| StreamHandler[handlers.rs / handle_events_stream]
        Router -->|Ack Endpoint| AckHandler[handlers.rs / handle_telemetry_ack]
        Router -->|Metrics Endpoint| MetricsHandler[handlers.rs / handle_metrics]
    end

    %% Frontend App
    subgraph Frontend Client (chronos-pilot)
        EventSrc[App.tsx / EventSource] -->|Consume SSE Stream| Rebuild[rebuildState Reducer]
        Rebuild -->|Updates React State| UI[Pilot UI Views]
        UI -->|Render confirmation ACK| PostAck[App.tsx / fetch /api/telemetry/ack]
        UI -->|Renders alerts ledger| TelemetryHUD[TelemetryConsole.tsx]
    end

    %% Scrapers
    Prometheus[Prometheus / Scraper] -->|Polls /metrics| MetricsHandler
    Grafana[Grafana / Dashboard] -->|Visualize performance| Prometheus
    Bus -->|Publish events| StreamHandler
    PostAck -->|Writes UiTelemetryAckReceived| AckHandler
    AckHandler -->|Appends acknowledgement| Store
```

---

## ─── FRONTEND FILE INVENTORY (chronos-pilot) ───

### Top-Level & Configuration Files

#### 1. [chronos-pilot/server.ts](file:///D:/Chronos_Hackathon/chronos-pilot/server.ts)
* **Role**: Staging & Dev Reverse Proxy and Asset Server.
* **Connected Backend**: Connects directly to `chronos-daemon` on port `7899` (bind address configurable via `CHRONOS_API_HOST`).
* **Functionality**:
  - In production mode (`NODE_ENV=production`), serves compiled static SPA files from the `dist/` directory.
  - In development mode, initializes a local Vite dev server middleware to enable hot-reloading.
  - Proxies all client requests hitting `/api` down to the Rust API bridge server.

#### 2. [chronos-pilot/vite.config.ts](file:///D:/Chronos_Hackathon/chronos-pilot/vite.config.ts)
* **Role**: Build Tool and Bundle Packager.
* **Functionality**: Configures bundling settings, entrypoints, and TypeScript modules for the build runtime.

#### 3. [chronos-pilot/Dockerfile](file:///D:/Chronos_Hackathon/chronos-pilot/Dockerfile)
* **Role**: Containerization Specification.
* **Functionality**: Standardizes staging and production environments using a Node base image to compile client assets and execute the Express server.

---

### Core Source Code (`src/`)

#### 4. [chronos-pilot/src/main.tsx](file:///D:/Chronos_Hackathon/chronos-pilot/src/main.tsx)
* **Role**: DOM Mount Point.
* **Functionality**: Bootstraps the React framework, inserting the main `App` layout container into the browser index template.

#### 5. [chronos-pilot/src/App.tsx](file:///D:/Chronos_Hackathon/chronos-pilot/src/App.tsx)
* **Role**: Primary Client Orchestrator & State Projector.
* **Connected Backend**: Consumes the `/api/events/stream/live` SSE channel and pushes render acknowledgements to `/api/telemetry/ack`.
* **Functionality**:
  - Establishes a persistent `EventSource` connection to feed the real-time event pipeline to the client.
  - Implements the `rebuildState` event-sourcing projection engine to reconstruct the active state models (commitments, actions, and projects) without fetching static snapshot models.
  - Controls telemetry loops, tracking out-of-order frames, network transit lag, and dropped sequences.

#### 6. [chronos-pilot/src/types.ts](file:///D:/Chronos_Hackathon/chronos-pilot/src/types.ts)
* **Role**: Core TypeScript Interface Declarations.
* **Functionality**: Implements schemas matching the backend models (e.g. `ChronosEvent`, `Commitment`, `ProjectAction`, `ProjectState`).

#### 7. [chronos-pilot/src/index.css](file:///D:/Chronos_Hackathon/chronos-pilot/src/index.css)
* **Role**: CSS Styling Layer.
* **Functionality**: Defines structural layout guidelines and themes.

---

### UI Component Files (`src/components/`)

#### 8. [chronos-pilot/src/components/TelemetryConsole.tsx](file:///D:/Chronos_Hackathon/chronos-pilot/src/components/TelemetryConsole.tsx)
* **Role**: System Telemetry & Alert Stream Dashboard.
* **Functionality**:
  - Renders live sequence metrics, dropped count counters, and phase-by-phase execution timing progress bars.
  - Renders a structured Alerts ledger table highlighting warning/critical anomalies (e.g. slow ticks or network delays) correlated with exact `tick_id` parameters.
  - Implements the "Replay History" simulation.

#### 9. [chronos-pilot/src/components/WorkspaceRestorer.tsx](file:///D:/Chronos_Hackathon/chronos-pilot/src/components/WorkspaceRestorer.tsx)
* **Role**: Workspace Reconstruction Interface.
* **Connected Backend**: Routes triggers to `/api/workspace/restore`.
* **Functionality**: Displays snapshots and allows manual restoration triggers to spin up ghost terminal contexts.

#### 10. [chronos-pilot/src/components/ARCPanel.tsx](file:///D:/Chronos_Hackathon/chronos-pilot/src/components/ARCPanel.tsx)
* **Role**: Action Resolution Console Panel.
* **Connected Backend**: Triggers actions via `/api/action/resolve`.
* **Functionality**: Displays the list of pending cognitive execution actions.

#### 11. [chronos-pilot/src/components/CommitmentList.tsx](file:///D:/Chronos_Hackathon/chronos-pilot/src/components/CommitmentList.tsx)
* **Role**: Commitments Tracker.
* **Functionality**: Renders obligation cards, inferred deadlines, and priority rankings.

#### 12. [chronos-pilot/src/components/RiskForecaster.tsx](file:///D:/Chronos_Hackathon/chronos-pilot/src/components/RiskForecaster.tsx)
* **Role**: Risk Analysis Visualization.
* **Functionality**: Renders live risk projection curves and attention decay timelines derived from `RiskForecastResolved` events.

#### 13. [chronos-pilot/src/components/DatabaseViewer.tsx](file:///D:/Chronos_Hackathon/chronos-pilot/src/components/DatabaseViewer.tsx)
* **Role**: SQLite Store Explorer.
* **Connected Backend**: Resolves snapshot queries from `/api/database`.
* **Functionality**: Renders tabular raw table registers directly from the persistent SQLite instance.

#### 14. [chronos-pilot/src/components/InteractiveSandbox.tsx](file:///D:/Chronos_Hackathon/chronos-pilot/src/components/InteractiveSandbox.tsx)
* **Role**: Perception Ingestion Sandbox.
* **Connected Backend**: Ingests custom events via `/api/ingest`.
* **Functionality**: Provides a test layout for mock event ingestion (VSCode active file, browser tab focus, clipboard snippets) to trigger runtime loops manually.

---

### UI Data Files (`src/data/`)

#### 15. [chronos-pilot/src/data/mockDB.ts](file:///D:/Chronos_Hackathon/chronos-pilot/src/data/mockDB.ts)
* **Role**: Mock Database Fallback Registry.
* **Functionality**: Houses mock configurations used only as a safety fallback when the SSE stream is disconnected.

---

## ─── CONNECTED BACKEND FILE INVENTORY ───

These files in the Rust daemon are responsible for serving the APIs consumed by `chronos-pilot`.

### API & Routing Layer (`chronos-api-bridge`)

#### 16. [chronos-api-bridge/src/router.rs](file:///D:/Chronos_Hackathon/chronos-api-bridge/src/router.rs)
* **Role**: API Route Definitions.
* **Functionality**: Declares endpoints for event streaming (`/api/events/stream`), database checks (`/api/database`), telemetry confirmations (`/api/telemetry/ack`), and Prometheus metric exposition (`/metrics`).

#### 17. [chronos-api-bridge/src/handlers.rs](file:///D:/Chronos_Hackathon/chronos-api-bridge/src/handlers.rs)
* **Role**: Route Action Controllers.
* **Functionality**:
  - Implements the Server-Sent Events (SSE) broadcaster stream translating memory bus events to client-parseable chunks.
  - Implements telemetry confirmations inserting `UiTelemetryAckReceived` events to database ledgers.
  - Runs SQL aggregations to calculate live Prom metrics (/metrics).

#### 18. [chronos-api-bridge/src/state.rs](file:///D:/Chronos_Hackathon/chronos-api-bridge/src/state.rs)
* **Role**: API Core State Provider.
* **Functionality**: Packages database connectors (`store`), registries, and active memory sessions inside `BridgeState` shared across all Axum routing handlers.

---

### Persistent Store Layer (`chronos-store-sqlite`)

#### 19. [chronos-store-sqlite/src/lib.rs](file:///D:/Chronos_Hackathon/chronos-store-sqlite/src/lib.rs)
* **Role**: SQLite persistence adapter.
* **Functionality**: Exposes the sqlite connection handle and stores warning alerts within the `chronos_alerts` schema table, queried during `/metrics` generation.
