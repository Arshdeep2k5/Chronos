# TECHNICAL_DEBT_REGISTER.md
*Authoritative Technical Debt & Architectural Risk Register*

---

## 1. Active Technical Debt Register

The following register documents the architectural debt, risks, and dependencies identified in the Chronos repository:

### DEBT-01: Double Backend Duality (Monolith vs PCOS Daemon)
*   **Description**: The codebase contains two independent, overlapping Rust backends: `src-tauri` (Tauri App with Axum HTTP server in `server.rs`) and `chronos-daemon` (Modular PCOS loop runtime with API bridge server in `chronos-api-bridge`).
*   **Cause**: The application is in transition from a monolithic prototype to a modular event-sourced architecture.
*   **Severity**: **Critical**.
*   **Impact**: Modifying database schemas or processing logic requires manual porting across both backends, leading to code drift.
*   **Recommended Resolution**: Deprecate `src-tauri/src/server.rs` and configure the Tauri app UI to communicate directly with `chronos-daemon` over port 7899.
*   **Blocking Future Work**: **Yes** (blocks database cleanups and feature consolidations).

### DEBT-02: Concurrent SQLite File Access (Lock Risks)
*   **Description**: Both the legacy Tauri backend and `chronos-daemon` access `chronos_events.db` and `chronos.db` directly on disk using hardcoded file paths.
*   **Cause**: Lack of a centralized database manager process.
*   **Severity**: **High**.
*   **Impact**: When both daemons are active, database write operations fail with `database is locked` errors.
*   **Recommended Resolution**: Restrict database write access to the modular `chronos-daemon` process, forcing other clients to read/write state over REST endpoints.
*   **Blocking Future Work**: **Yes** (blocks stability under multi-process usage).

### DEBT-03: Local Event Sourcing Parsing in UI (Client-Side Parsing)
*   **Description**: The `DatabaseViewer.tsx` component parses client-side event arrays inside React instead of querying SQLite tables directly.
*   **Cause**: Simplification of early UI components to bypass database queries.
*   **Severity**: **Medium**.
*   **Impact**: Changes to event structures or timeline pruning operations cause table rendering drift in the dashboard.
*   **Recommended Resolution**: Implement API bridge endpoints that query SQLite tables directly and return structured database representations to the UI.
*   **Blocking Future Work**: **No**.

### DEBT-04: Empty Crate Stub (`chronos-commitment-inference-engine`)
*   **Description**: The `chronos-commitment-inference-engine` crate contains only a `Cargo.toml` file with no `src/` directory.
*   **Cause**: Incomplete migration of commitment logic.
*   **Severity**: **Low**.
*   **Impact**: Dead crate dependency in workspace, with commitment heuristics still coupled to legacy monolith codes.
*   **Recommended Resolution**: Migrate commitment heuristics to the workspace crate and remove the legacy code references.
*   **Blocking Future Work**: **No**.

---
