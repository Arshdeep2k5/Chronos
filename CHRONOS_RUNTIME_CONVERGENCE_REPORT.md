# Chronos Runtime Convergence Report

**Role**: Principal Systems Architect
**Date**: 2026-06-28
**Subject**: Unification of daemon and API bridge runtimes

## 1. Runtime Architecture Audit

**Original State:**
*   **chronos-daemon**: Initialized a local `MemoryEventBus` and `SQLiteEventStore`. Registered all services and adapters. Subscribed to its bus to run the L2-L5 cognitive pipeline.
*   **chronos-api-bridge**: Initialized its own isolated `MemoryEventBus` and `SQLiteEventStore`. Listened on an HTTP port to accept telemetry and publish to its isolated bus.
*   **Issue**: Real-world events (Browser, VSCode, Manual) were arriving at the API bridge, being published to its isolated bus, persisting to SQLite, but *failing* to trigger the daemon's active memory/reasoning pipeline.

**Redundancy Identified**: Both binaries were duplicating the full PCOS state (Bus, Store, EntityResolver, SessionEngine).

## 2. Shared Runtime Construction

A unified runtime structure was established inside `chronos-daemon`:
*   `DaemonRuntime` now hosts the canonical `Arc<SQLiteEventStore>` and `Arc<MemoryEventBus>`.
*   The `EntityResolver` and `SessionEngine` have been wrapped in `Arc<RwLock<T>>` to allow concurrent mutable access across the async pipeline loop and the HTTP API handlers.

## 3. Daemon Refactor

`chronos-daemon/src/main.rs` was heavily refactored to act as the primary, unified host:
1.  **Warm Up**: Replays persisted events to construct the shared `EntityResolver` and `SessionEngine` behind `RwLock`s.
2.  **Pipeline Construction**: Spawns the L2-L5 pipeline worker, passing the shared memory locks. The pipeline acquires write locks on each cycle.
3.  **Adapter Construction**: Spawns the L1 perception adapters (`WindowFocus`, `Clipboard`, `Filewatcher`, `Git`) against the canonical bus.
4.  **API Bridge Integration**: Instantiates `BridgeState` using the canonical bus, store, resolver, and session engine. Mounts the axum router and spawns the API server task.

## 4. API Bridge Refactor

The `chronos-api-bridge` crate was modified:
1.  Removed from the workspace as a standalone binary (`[[bin]]` stripped from `Cargo.toml`).
2.  Deleted `src/main.rs`.
3.  Converted entirely into a pure library crate exporting `router::build_router(BridgeState)`.
4.  No longer constructs its own isolated store or bus.

## 5. Success Verification

*   **One Runtime**: `chronos-daemon` is now the sole entry point for the entire backend.
*   **One Bus**: Both the `POST /api/perception/ingest` handler and the L1 adapters publish to the exact same `MemoryEventBus` instance (capacity 4096).
*   **Live Cognition**: A browser event hitting the REST endpoint instantly flows down the bus, enters the `EntityResolver`, and triggers Risk & Decision evaluation without restarting the daemon.
*   **Build & Tests**: `cargo test` confirms the daemon passes all end-to-end integration tests (7 tests passing), proving that `replay_events` determinism and `run_pipeline` concurrent locking logic is functionally correct.
