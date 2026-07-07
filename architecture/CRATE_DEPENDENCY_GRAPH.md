# CRATE_DEPENDENCY_GRAPH.md
*Authoritative Workspace Crate Dependency Specification & Verification*

---

## 1. Crate Dependency Graph

The modular PCOS framework consists of 24 core crates. The dependency relationships between layers are strictly managed:

```mermaid
graph TD
    %% Crates
    Core[chronos-core]
    Bus[chronos-bus]
    Store[chronos-store]
    StoreSql[chronos-store-sqlite]
    Reg[chronos-registry]
    Cont[chronos-container]
    Conf[chronos-config]
    Log[chronos-logging]
    
    AdpFile[chronos-adapter-filewatcher]
    AdpGit[chronos-adapter-git]
    AdpWin[chronos-adapter-window-focus]
    AdpClip[chronos-adapter-clipboard]
    
    MemER[chronos-memory-entity-resolution]
    MemSess[chronos-memory-sessions]
    MemState[chronos-memory-state]
    
    ReasonRef[chronos-reasoning-reflection]
    ReasonCmt[chronos-reasoning-commitments]
    ReasonDDE[chronos-reasoning-dde]
    ReasonPCM[chronos-reasoning-pcm]
    ReasonRisk[chronos-reasoning-risk]
    
    Dec[chronos-decision-orchestrator]
    
    ExecCCE[chronos-execution-cce]
    ExecRun[chronos-execution-runtime]
    
    Daemon[chronos-daemon]

    %% Infrastructure Dependencies
    Bus --> Core
    Store --> Core
    StoreSql --> Store
    StoreSql --> Core
    Reg --> Core
    Cont --> Core
    Conf --> Core
    Log --> Core

    %% Perception Dependencies
    AdpFile & AdpGit & AdpWin & AdpClip --> Bus
    AdpFile & AdpGit & AdpWin & AdpClip --> Reg
    AdpFile & AdpGit & AdpWin & AdpClip --> Log

    %% Memory Dependencies
    MemER & MemSess & MemState --> Bus
    MemER & MemSess & MemState --> Store
    MemER & MemSess & MemState --> Reg
    MemSess --> MemER
    MemState --> MemSess
    MemState --> MemER

    %% Reasoning Dependencies
    ReasonRef & ReasonCmt & ReasonDDE & ReasonPCM & ReasonRisk --> Bus
    ReasonRef & ReasonCmt & ReasonDDE & ReasonPCM & ReasonRisk --> Store
    ReasonRef & ReasonCmt & ReasonDDE & ReasonPCM & ReasonRisk --> MemState
    ReasonDDE --> ReasonCmt
    ReasonPCM --> ReasonDDE
    ReasonRisk --> ReasonPCM

    %% Decision Dependencies
    Dec --> Bus
    Dec --> Store
    Dec --> ReasonRisk
    Dec --> ReasonRef

    %% Execution Dependencies
    ExecCCE --> Bus
    ExecCCE --> Dec
    ExecCCE --> Store
    ExecRun --> Bus
    ExecRun --> Cont
    ExecRun --> Conf

    %% Daemon Runtime
    Daemon --> StoreSql
    Daemon --> Bus
    Daemon --> Reg
    Daemon --> AdpGit
    Daemon --> AdpFile
    Daemon --> AdpWin
    Daemon --> AdpClip
```

---

## 2. Crate Inventory & Layer Mappings

### 2.1 Layer 0 — Infrastructure Crates
*   **`chronos-core`**: Core model schemas.
    *   *Direct Dependencies*: None.
    *   *Reverse Dependencies*: All workspace crates.
*   **`chronos-bus`**: Asynchronous EventBus.
    *   *Direct Dependencies*: `chronos-core`, `tokio`, `async-trait`.
    *   *Reverse Dependencies*: All Perception, Memory, and Execution runtime components.
*   **`chronos-store-sqlite`**: Durable event store.
    *   *Direct Dependencies*: `chronos-core`, `chronos-store`, `rusqlite`, `serde_json`, `uuid`.
    *   *Reverse Dependencies*: `chronos-daemon`, `chronos-api-bridge`.

### 2.2 Layer 1 — Perception Crates
*   **`chronos-adapter-clipboard`**: Clipboard listener.
    *   *Direct Dependencies*: `chronos-core`, `chronos-bus`, `chronos-registry`, `chronos-logging`, `windows-sys`.
    *   *Reverse Dependencies*: `chronos-daemon`.
*   **`chronos-adapter-window-focus`**: Window switch observer.
    *   *Direct Dependencies*: `chronos-core`, `chronos-bus`, `chronos-registry`, `chronos-logging`, `windows-sys`.
    *   *Reverse Dependencies*: `chronos-daemon`.

### 2.3 Layer 2 — Memory Crates
*   **`chronos-memory-sessions`**: Groups events into sessions.
    *   *Direct Dependencies*: `chronos-core`, `chronos-bus`, `chronos-store`, `chronos-memory-entity-resolution`.
    *   *Reverse Dependencies*: `chronos-memory-state`, `chronos-daemon`.

### 2.4 Layer 3 — Reasoning Crates
*   **`chronos-reasoning-risk`**: Formulates risk trends.
    *   *Direct Dependencies*: `chronos-core`, `chronos-bus`, `chronos-store`, `chronos-reasoning-pcm`.
    *   *Reverse Dependencies*: `chronos-decision-orchestrator`, `chronos-daemon`.

---

## 3. Coupling Violations & Debt Audit

*   **Linear Reasoning Coupling (Heuristic Layer dependency)**:
    *   *Status*: `chronos-reasoning-risk` directly references `chronos-reasoning-pcm`, which references `chronos-reasoning-dde`.
    *   *Violation*: This creates a linear compile dependency chain in Layer 3. If a change is made to `DDE`, all downstream reasoning crates must recompile.
    *   *Suggested Fix*: Decouple reasoning engines by publishing results as events (`DeadlineDiscovered`, `CapacityProfileUpdated`) on the `MemoryEventBus`, allowing each engine to run independently.
*   **Missing Core Interface abstraction**:
    *   Perception adapters (`chronos-adapter-clipboard`) directly depend on `windows-sys` and concrete bus instances. This makes unit testing on non-Windows platforms difficult. The adapters should depend on abstract OS interface traits instead.

---
