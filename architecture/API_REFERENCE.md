# API_REFERENCE.md
*Authoritative API Endpoint Reference Specification*

---

## 1. Modular API Bridge (`chronos-api-bridge`) — Port 7899

Serves PCOS state and event streams to the React UI dashboard.

### 1.1 Cognitive Event Endpoints

#### `GET /api/events/stream`
*   **Purpose**: Retrieves historical chronos events.
*   **Request Parameters**: None.
*   **Response Body (JSON)**:
    ```json
    {
      "ok": true,
      "data": {
        "total": 12,
        "events": [
          {
            "id": "e6740684-2101-4475-b6d3-2ea47e2b1be7",
            "timestamp": "2026-06-29T10:12:30Z",
            "schema_version": "1.0.0",
            "event_type": "WindowFocusChanged",
            "source": "svc-adapter-window-focus",
            "payload": {
              "window_title": "Vim - master_architecture.md",
              "process_name": "nvim.exe"
            }
          }
        ]
      }
    }
    ```
*   **Implementation Status**: Fully Implemented.

#### `GET /api/events/stream/live`
*   **Purpose**: Establishes a Server-Sent Events (SSE) connection to stream live events as they are published to the `MemoryEventBus`.
*   **Headers Required**: `Accept: text/event-stream`.
*   **Payload Output**: Data chunks prefixed with `data: ` containing serialized JSON representations of `ChronosEvent`.
*   **Implementation Status**: Fully Implemented.

### 1.2 Cognitive State & Diagnostics Endpoints

#### `GET /api/state`
*   **Purpose**: Returns the most recently materialized global world model snapshot.
*   **Response Body (JSON)**:
    ```json
    {
      "ok": true,
      "data": {
        "state_id": "06716027-21a4-4fa9-b883-fa496a1a11ff",
        "active_commitment_ids": [],
        "priority_vector": [],
        "risk_snapshot": 0.0,
        "coherence_score": 1.0,
        "intent_summary": [],
        "unresolved_conflict_count": 0,
        "synthesized_at": "2026-06-29T10:12:30Z"
      }
    }
    ```
*   **Implementation Status**: Fully Implemented (idle state returns empty vectors).

#### `GET /api/reasoning/forecasts`
*   **Purpose**: Retrieves current failure probabilities and risk decay trajectories.
*   **Response Body (JSON)**:
    ```json
    {
      "ok": true,
      "data": {
        "status": "idle",
        "forecasts": []
      }
    }
    ```
*   **Implementation Status**: Fully Implemented.

### 1.3 Execution Plan & Restoration Endpoints

#### `POST /api/execution/generate-recovery-plan`
*   **Purpose**: Requests a step-by-step checklist to recover an at-risk commitment.
*   **Request Body (JSON)**:
    ```json
    {
      "commitment_id": "c-101a"
    }
    ```
*   **Response Body (JSON)**:
    ```json
    {
      "ok": true,
      "data": {
        "plan_id": "p-102",
        "steps": [
          "Verify local branch state",
          "Pull latest changes from origin",
          "Resolve merge conflicts in layout.tsx"
        ]
      }
    }
    ```
*   **Implementation Status**: Fully Implemented.

#### `POST /api/execution/restore-workspace`
*   **Purpose**: Restores local IDE file tabs and cursor positions.
*   **Request Body (JSON)**:
    ```json
    {
      "snapshot_id": "ws-snap-10"
    }
    ```
*   **Response Body (JSON)**:
    ```json
    {
      "ok": true,
      "message": "Workspace restoration dispatched successfully."
    }
    ```
*   **Implementation Status**: Fully Implemented (sends `WORKSPACE_RESTORE` command to active VSCode extension WebSocket).

---

## 2. Telemetry Ingestion (OS Integrations) — Port 48120 - 48123

Handled by the WebSockets listener of the Tauri monolith.

### 2.1 Browser WebSocket Channel (`/telemetry/browser`)
*   **Protocol**: WebSocket (`ws://`).
*   **Purpose**: Receives real-time browser navigation and web searches from the browser extension.
*   **Payload Types**:
    *   `TAB_FOCUS`: Sent when tab changes.
    *   `SEARCH_QUERY`: Sent when search engines query strings.
*   **Response Events**: Emits `DISTRACTION_INTERCEPT` to trigger client-side friction overlay blocks.

### 2.2 IDE WebSocket Channel (`/telemetry/ide`)
*   **Protocol**: WebSocket (`ws://`).
*   **Purpose**: Receives cursor and window layout telemetry from the editor.
*   **Payload Types**:
    *   `WORKSPACE_TELEMETRY`: JSON payload detailing open tabs paths, line numbers, and cursor focus.
*   **Response Events**: Emits `WORKSPACE_RESTORE` JSON payload containing files lists to restore layout.

---
