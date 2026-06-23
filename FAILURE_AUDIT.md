# Failure Audit & Contingency Plan: Chronos Pilot (v1.0)

This plan details direct mitigation actions for edge cases, failures, and latency exceptions during live judging scenarios.

## 1. Local Network Port Binding Collisions
* **Failure Mode**: The Core Daemon fails to bind to port `48120` because it is already occupied.
* **Diagnostic**: Tauri displays a connection retry loop indefinitely on boot.
* **Contingency Route**:
  1. The Rust Daemon automatically drops down to scan a port pool array: `[48120, 48121, 48122, 48123]`.
  2. The assigned port is immediately written to the local handshake payload file `handshake.json`.
  3. External connectors dynamically check `handshake.json` to resolve active target ports before establishing connection parameters.

## 2. Browser Extension Connection Dropout
* **Failure Mode**: The browser extension loses loopback socket connectivity, stops streaming telemetry events, or fails to load.
* **Diagnostic**: Browser events stop appearing inside the `context_events` table; the HUD stops updating active session statuses.
* **Contingency Route**:
  1. Rust Daemon automatically monitors extension heartbeats.
  2. If the connection fails, the Daemon falls back to a fallback poll loop executing every 10 seconds.
  3. It reads the local browser session history files directly from disk (`Library/Application Support/Google/Chrome/Default/History` on macOS or `%LocalAppData%/Google/Chrome/User Data/Default/History` on Windows).
  4. Parsed history entries are mapped directly into the database schema tables.

## 3. VSCode Workspace Restoration Failure
* **Failure Mode**: The VSCode WebSocket connection fails to restore, or shell scripts fail to execute programmatic layout configurations.
* **Diagnostic**: Clicking "Start Working" triggers a file-not-found error or does not load VSCode tabs.
* **Contingency Route**:
  1. Wrap VSCode script initialization with system shell command exceptions.
  2. If raw socket restoration fails, fall back to executing standard system terminal spawn commands: `code --folder-uri <project_path>`.
  3. This commands VSCode to load the target directory, allowing VSCode's default workspace caching state to restore open editors.

## 4. Python Subprocess / NLP Crash
* **Failure Mode**: The local Python subprocess crashes during high-volume document ingestion.
* **Diagnostic**: Dead Letter Queue displays retry triggers; the HUD diagnostic summaries stop updating.
* **Contingency Route**:
  1. Rust catches Python exit signals and attempts a maximum of 3 restarts.
  2. If execution crashes persistent files, write the source file paths to the `dead_letter_queue` table and ignore them.
  3. Drop down to standard regex text parsing logic executed directly in Rust to parse deadlines and priority keys, bypassing local vector dependencies.
