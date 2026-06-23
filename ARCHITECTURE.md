# Architecture Specification: Chronos Pilot (v1.0)

This document specifies the technical boundaries, component interactions, runtime environment, and process lifecycles of Chronos Pilot.

## 1. Component Topology Diagram

```text
       ┌────────────────────────────────────────────────────────┐
       │                 BROWSER EXTENSION (MV3)                │
       │   - Captures Active URL, Page Title, Search Queries    │
       │   - Tracks focus duration via active tab state timers   │
       └───────────────────────────┬────────────────────────────┘
                                   │
                                   │ (Local Loopback HTTP / WS + Token)
                                   ▼
┌──────────────────────────────────────────────────────────────────────┐
│                        TAURI DESKTOP RUNTIME                         │
│                                                                      │
│  ┌──────────────────────┐               ┌─────────────────────────┐  │
│  │ SolidJS UI Frontend  │               │   Rust Core Daemon      │  │
│  │ - HUD Interface      │  (Tauri IPC)  │   - tokio Event Loop    │  │
│  │ - Simulation Charts  │◄─────────────►│   - notify FS Watcher   │  │
│  │ - Recovery Checklist │               │   - active-win win-pos  │  │
│  └──────────────────────┘               └────────────┬────────────┘  │
└──────────────────────────────────────────────────────║───────────────┘
                                                       ║
                    ┌──────────────────────────────────┴──────────────────────────────────┐
                    │ (Secure Local Sockets IPC / Heartbeat Monitor)                      │
                    ▼                                                                     ▼
┌──────────────────────────────────────┐                               ┌──────────────────────────────────────┐
│        PYTHON AI WORKER PROCESS      │                               │        VSCODE / IDE CONNECTOR        │
│  - SQLite-vec Embeddings Generation  │                               │  - Extension active tab state        │
│  - Local LLM Narrative Generator     │                               │  - Line/Column cursor tracking       │
│  - CDE Parser Engine & DLQ Handler   │                               │  - Programmatic session reconstruction│
└──────────────────────────────────────┘                               └──────────────────────────────────────┘
```

## 2. Process Lifecycle & Process Management

### 2.1 Boot Sequence
1. **Launch**: User runs the Tauri desktop binary.
2. **Core Daemon Init**: The Rust process initializes and verifies the local database file `chronos.db`. If missing, it applies SQLite migrations.
3. **Local Port Binding**: Rust binds a dedicated local port (`localhost:48120`) with a dynamically generated cryptographic token written to `~/.config/chronos/handshake.json`.
4. **Subprocess Spawning**: Rust spawns the Python worker process:
   * Spawns `/usr/bin/python3` passing the embedded workers directory.
   * Allocates unique OS process groups to guarantee termination boundaries.
5. **Telemetry Activation**: Rust spawns the filesystem monitor (`notify` loop) and starts listening for incoming WS connections from the Browser Extension and VSCode Connector.

### 2.2 Process Monitoring & Heartbeat Loop
The Rust Daemon acts as the parent controller. Every 5 seconds, the Python worker must send a JSON payload over loopback HTTP:

```json
{"status": "ALIVE", "worker": "python_nlp", "memory_mb": 114}
```

* **Missed Heartbeats**: If the Daemon misses 3 consecutive heartbeats (15s), the Daemon forcefully kills the target process ID via `SIGKILL` (or `TerminateProcess` on Windows).
* **Crash Threshold**: Rust attempts a restart. If the worker crashes more than 3 times within a rolling 10-minute window, the Daemon enters `FAILED_STATE`, halts further restarts, and displays a degraded state notification on the SolidJS HUD.

### 2.3 Shutdown Sequence
1. **Signal Interception**: Tauri traps the application exit event or OS termination signals (`SIGINT`, `SIGTERM`).
2. **Subprocess Termination**: Rust terminates the Python process group and closes database handles.
3. **Connection Teardown**: Closes open loopback ports.
4. **Exit**: Parent exit code is emitted cleanly.

## 3. Threading Model (Rust Core Daemon)
* **Main Thread (UI Execution)**: Executes the Tauri webview and window rendering loops.
* **Worker Threadpool (`tokio` runtime)**:
  * **Thread 1 (FS Watcher)**: Listens to raw events from `notify` and routes changed files to the ingestion queue.
  * **Thread 2 (Windows Focus Watcher)**: Polls active windows every 1.0 seconds via `active-win-pos-rs` to track user application swaps.
  * **Thread 3 (Network Loopback Server)**: An asynchronous axum/hyper HTTP and WebSocket server handling browser, IDE, and UI telemetry.
  * **Thread 4 (Ingestion & DB Writer)**: A single-threaded channel dedicated to executing serialized write operations against the SQLite database to avoid concurrency lockups.

## 4. Subsystem Communications

### 4.1 UI communication (Tauri IPC)
Tauri uses its standard secure asynchronous IPC bridge, exposing native Rust methods to SolidJS via commands:
* `invoke("trigger_restore", { projectId: 1 })`
* Rust pushes asynchronous updates (e.g., `PROJECT_RISK_UPDATE`) using `Window::emit()`.

### 4.2 Browser & IDE Communication (WebSocket Server)
The local loopback server uses strict validation constraints:
* Every request must pass the authorization header containing the cryptographic token loaded from `handshake.json`.
* Non-local requests are rejected instantly at the socket binding level.
