# RUNTIME_SEQUENCE.md
*Authoritative System Startup & Runtime Lifecycle Sequence Specification*

---

## 1. Subsystem Boot Order

The system follows a strict order when initializing components:

```mermaid
sequenceDiagram
    autonumber
    participant Host as Host Environment
    participant Daemon as chronos-daemon (main)
    participant DB as SQLite Event Store
    participant Bus as MemoryEventBus
    participant Reg as ServiceRegistry
    participant Pipes as Pipeline Workers
    participant Adp as Perception Adapters
    participant API as API Bridge (Axum)

    Host->>Daemon: Process Spawned (chronos-daemon.exe)
    activate Daemon
    Daemon->>Daemon: Load ENV & Logging Config
    
    Daemon->>DB: Open chronos_events.db (SQLite)
    activate DB
    DB-->>Daemon: Connection Pool & WAL initialized
    deactivate DB

    Daemon->>Bus: Instantiate MemoryEventBus (tokio::sync::broadcast)
    activate Bus
    Bus-->>Daemon: Bus initialized (capacity: 4096)
    deactivate Bus

    Daemon->>Reg: Instantiate ServiceRegistry
    activate Reg
    Reg->>Reg: Register svc-event-store-sqlite & svc-cognitive-bus
    Reg-->>Daemon: Registry initialized
    deactivate Reg

    Daemon->>DB: Read event stream history (replay query)
    activate DB
    DB-->>Daemon: Return History: Vec<ChronosEvent>
    deactivate DB

    Daemon->>Daemon: Warm up memory engines (EntityResolver & SessionEngine)

    Daemon->>Pipes: Spawn run_pipeline task
    activate Pipes
    Pipes->>Bus: Subscribe to MemoryEventBus
    Pipes-->>Daemon: Pipeline processing active
    deactivate Pipes

    Daemon->>Adp: Start Layer 1 perception loops (Clipboard, Window, Files, Git)
    activate Adp
    Adp->>Reg: Register Adapter Status
    Adp-->>Daemon: OS Observer Threads Active
    deactivate Adp

    Daemon->>API: Spawn Axum Serve (Port 7899)
    activate API
    API-->>Daemon: HTTP Router Operational
    deactivate API

    Daemon-->>Host: Daemon Fully Operational (SIGTERM / Ctrl+C await)
    deactivate Daemon
```

---

## 2. Service Registration & Dependency Injection

PCOS uses an IoC Container (`chronos-container`) to register and resolve singletons.
1.  **Registry Binding**: Every engine (e.g., `svc-commitment-engine`, `svc-decision-orchestrator`) registers its `ServiceDescriptor` with `ServiceRegistry` at boot.
2.  **Singleton Resolution**: Downstream processing loops retrieve dependencies by querying `Container::get::<T>()`.

---

## 3. The 6-Phase Cognitive Tick Loop

The processing pipeline is governed by a synchronous, transactional loop inside `chronos-runtime-loop` (Continuous Runtime Loop Engine):

```mermaid
sequenceDiagram
    autonumber
    participant Bus as MemoryEventBus
    participant CRLE as CRLE (execute_tick)
    participant Memory as Layer 2: Memory Engines
    participant Reason as Layer 3: Reasoning Engines
    participant Dec as Layer 4: Decision Orchestrator
    participant Exec as Layer 5: Execution Runtime
    participant Store as SQLite EventStore

    Bus->>CRLE: Emit ChronosEvent
    activate CRLE
    
    Note over CRLE: Phase 1: Ingest Events
    CRLE->>Memory: Process events
    activate Memory
    Memory-->>CRLE: Updated Entity Graph & Sessions
    deactivate Memory

    Note over CRLE: Phase 2: Run Reasoning Engines
    CRLE->>Reason: Evaluate State, Deadlines & Risks
    activate Reason
    Reason-->>CRLE: Resolve Commitments, Deadlines & Capacity Profiles
    deactivate Reason

    Note over CRLE: Phase 3: Synthesize CognitiveState
    CRLE->>CRLE: Compute global coherence_score & risk_snapshot
    
    Note over CRLE: Phase 4: Generate Decision
    CRLE->>Dec: Evaluate warrants_decision()
    activate Dec
    Dec-->>CRLE: Resolve ChronosDecision (Notify / WorkspaceRestore / NoAction)
    deactivate Dec

    Note over CRLE: Phase 5: Execute Plan
    CRLE->>Exec: Dispatch action commands
    activate Exec
    Exec-->>CRLE: Action Feedback (Started / Completed)
    deactivate Exec

    Note over CRLE: Phase 6: Collect Feedback
    CRLE->>Store: Persist TickFrame telemetry
    activate Store
    Store-->>CRLE: Tick Transaction Committed
    deactivate Store

    CRLE-->>Bus: Publish processed outcomes (DecisionResolved / ActionStarted)
    deactivate CRLE
```

---

## 4. Graceful Shutdown Order

Upon receiving `SIGINT` (Ctrl+C) or `SIGTERM`:
1.  **Adapter Termination**: Perception adapters exit Win32 hooks and file-watching threads.
2.  **Pipeline Flush**: The run_pipeline thread processes remaining events in the `MemoryEventBus` queue.
3.  **Database Commit**: Event serialization is flushed, transaction locks are released, and the connection pool closes.
4.  **Process Exit**: Clean exit code (0) is returned to the host OS.

---
