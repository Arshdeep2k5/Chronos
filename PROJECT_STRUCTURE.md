# CHRONOS — FULL PROJECT STRUCTURE
> Generated: 2026-06-29 | Excludes: `.git/`, `target/`, `node_modules/`, `__pycache__/`, `dist/`

---

```
D:\Chronos_Hackathon\
│
├── .gitignore
├── .agents\
│   └── AGENTS.md                          # Agent behaviour rules
│
├── build.bat                              # Build script
├── stress_test.py                         # Stress test runner
│
│── PROJECT_STRUCTURE.md                   # ← This file
│
├── ── DOCUMENTATION ──────────────────────────────────────────────────
│
├── ARCHITECTURE.md
├── BOUNDED_TICK_PROCESSING_LIMITS.md
├── CCE_ENGINE_SPEC.md
├── CCP_SPECIFICATION.md
├── CHRONOS_API_BRIDGE_REPORT.md
├── CHRONOS_COGNITIVE_SESSION_SPECIFICATION.md
├── CHRONOS_DAEMON_SPEC.md
├── CHRONOS_INTEGRATION_REPORT.md
├── CHRONOS_INTEGRATION_VERIFICATION_AUDIT.md
├── CHRONOS_LIVE_EVENT_FLOW_VALIDATION.md
├── CHRONOS_OBJECT_MODEL.md
├── CHRONOS_PERCEPTION_WIRING_VERIFICATION.md
├── CHRONOS_RETROSPECTIVE_AUDIT.md
├── CHRONOS_RUNTIME_CONVERGENCE_REPORT.md
├── CHRONOS_RUNTIME_CONVERGENCE_VERIFICATION_AUDIT.md
├── CHRONOS_RUNTIME_VALIDATION.md
├── CHRONOS_STABILITY_AUDIT.md
├── CHRONOS_STATE_PROJECTOR_SPEC.md
├── CHRONOS_STATE_SPECIFICATION.md
├── CHRONOS_UI_MIGRATION_PLAN.md
├── CLIPBOARD_ADAPTER_SPEC.md
├── COGNITIVE_SESSION_ENGINE_SPEC.md
├── COMMITMENT_ENGINE_SPEC.md
├── DATABASE_SCHEMA.md
├── DEADLINE_ENGINE_SPEC.md
├── DECISION_ORCHESTRATOR_SPEC.md
├── DEMO_SCRIPT.md
├── DEPENDENCY_GRAPH.md
├── ENTITY_RESOLUTION_SPEC.md
├── EXECUTION_RUNTIME_SPEC.md
├── FAILURE_AUDIT.md
├── FILEWATCHER_ADAPTER_SPEC.md
├── GIT_ADAPTER_SPEC.md
├── IMPLEMENTATION_BOARD.md
├── INTERFACES.md
├── KERNEL_FREEZE_REVIEW.md
├── PCM_ENGINE_SPEC.md
├── PCOS_ARCHITECTURE.md
├── PRIVACY.md
├── REFLECTION_ENGINE_SPEC.md
├── RISK_ENGINE_SPEC.md
├── SQLITE_EVENT_STORE_SPEC.md
├── V3_UPGRADE_TODOS.md
├── WINDOW_FOCUS_ADAPTER_SPEC.md
├── implemented_features_summary.md
├── Chronos_Actuall Thoughts.txt
├── SRS_MVP_Vibe2Ship.txt
└── Vibe2Ship_SRS.txt
│
│
├── ── RUST BACKEND CRATES ─────────────────────────────────────────────
│
│
├── chronos-core\                          # FOUNDATION — Canonical data contracts
│   ├── Cargo.toml
│   ├── Cargo.lock
│   └── src\
│       └── lib.rs                         # ChronosEvent, ChronosState, ChronosIntent,
│                                          # ChronosDecision, ChronosAction,
│                                          # ChronosReflection, ChronosCapability
│
├── chronos-bus\                           # FOUNDATION — Cognitive pub/sub event bus
│   ├── Cargo.toml
│   ├── Cargo.lock
│   └── src\
│       └── lib.rs                         # EventBus trait, MemoryEventBus,
│                                          # MemorySubscriber, Publisher, Subscriber,
│                                          # BusError
│
├── chronos-config\                        # FOUNDATION — System configuration
│   ├── Cargo.toml
│   ├── Cargo.lock
│   └── src\
│       └── lib.rs
│
├── chronos-container\                     # FOUNDATION — Dependency injection container
│   ├── Cargo.toml
│   ├── Cargo.lock
│   └── src\
│       └── lib.rs
│
├── chronos-logging\                       # FOUNDATION — Structured logging
│   ├── Cargo.toml
│   ├── Cargo.lock
│   ├── README.md
│   └── src\
│       └── lib.rs
│
├── chronos-registry\                      # FOUNDATION — Service registry
│   ├── Cargo.toml
│   ├── Cargo.lock
│   └── src\
│       └── lib.rs
│
│
├── chronos-store\                         # STORAGE — Abstract event store trait
│   ├── Cargo.toml
│   ├── Cargo.lock
│   └── src\
│       └── lib.rs
│
├── chronos-store-sqlite\                  # STORAGE — SQLite event store (source of truth)
│   ├── Cargo.toml
│   ├── Cargo.lock
│   └── src\
│       └── lib.rs
│
│
├── chronos-memory-entity-resolution\      # MEMORY — Entity graph resolution
│   ├── Cargo.toml
│   ├── Cargo.lock
│   ├── README.md
│   └── src\
│       └── lib.rs                         # EntityResolver, EntityGraph
│
├── chronos-memory-sessions\               # MEMORY — Focus session tracking
│   ├── Cargo.toml
│   ├── Cargo.lock
│   ├── README.md
│   └── src\
│       └── lib.rs                         # SessionEngine, SessionProjection
│
├── chronos-memory-state\                  # MEMORY — Projected world state
│   ├── Cargo.toml
│   ├── Cargo.lock
│   ├── README.md
│   └── src\
│       └── lib.rs                         # ProjectedStatePayload
│
│
├── chronos-reasoning-commitments\         # REASONING — Commitment engine (592 lines)
│   ├── Cargo.toml
│   ├── Cargo.lock
│   ├── README.md
│   └── src\
│       └── lib.rs                         # CommitmentEngine, CommitmentState,
│                                          # Commitment, CommitmentCandidate,
│                                          # CommitmentStatus (Candidate/Active/
│                                          # AtRisk/Completed/Cancelled),
│                                          # all Commitment*Payload types
│
├── chronos-reasoning-intent\             # REASONING — Intent detection
│   ├── Cargo.toml
│   ├── Cargo.lock
│   ├── README.md
│   └── src\
│       └── lib.rs                         # CanonicalCommitmentBuilder
│
├── chronos-reasoning-coherence\          # REASONING — Cognitive coherence
│   ├── Cargo.toml
│   ├── Cargo.lock
│   ├── README.md
│   └── src\
│       └── lib.rs                         # CoherenceEngine::rebuild_cognitive_state(),
│                                          # reconcile()
│
├── chronos-reasoning-continuity\         # REASONING — Context continuity
│   ├── Cargo.toml
│   ├── Cargo.lock
│   ├── README.md
│   └── src\
│       └── lib.rs                         # ContextContinuityEngine
│
├── chronos-reasoning-decision\           # REASONING — Decision pipeline
│   ├── Cargo.toml
│   ├── Cargo.lock
│   ├── README.md
│   └── src\
│       └── lib.rs                         # DecisionPipeline::generate_decisions(),
│                                          # rebuild_decision_graph()
│
├── chronos-reasoning-dde\                # REASONING — Dynamic decision engine
│   ├── Cargo.toml
│   ├── Cargo.lock
│   ├── README.md
│   └── src\
│       └── lib.rs
│
├── chronos-reasoning-pcm\                # REASONING — Priority-context model
│   ├── Cargo.toml
│   ├── Cargo.lock
│   ├── README.md
│   └── src\
│       └── lib.rs
│
├── chronos-reasoning-reflection\         # REASONING — Learning & reflection
│   ├── Cargo.toml
│   ├── Cargo.lock
│   ├── README.md
│   └── src\
│       └── lib.rs
│
├── chronos-reasoning-risk\               # REASONING — Risk forecasting
│   ├── Cargo.toml
│   ├── Cargo.lock
│   ├── README.md
│   └── src\
│       └── lib.rs
│
│
├── chronos-execution-orchestration\      # EXECUTION — Execution orchestrator
│   ├── Cargo.toml
│   ├── Cargo.lock
│   ├── README.md
│   └── src\
│       └── lib.rs                         # ExecutionOrchestrator, ExecutionOutcome,
│                                          # OutcomeType,
│                                          # ExecutionOutcomeRecordedPayload
│
├── chronos-execution-cce\                # EXECUTION — Cognitive command executor
│   ├── Cargo.toml
│   ├── Cargo.lock
│   ├── README.md
│   └── src\
│       └── lib.rs
│
├── chronos-execution-feedback\           # EXECUTION — Feedback loop
│   ├── Cargo.toml
│   ├── Cargo.lock
│   ├── README.md
│   └── src\
│       └── lib.rs                         # FeedbackEngine::process_outcome()
│
├── chronos-execution-adapters\           # EXECUTION — Action adapter layer
│   ├── Cargo.toml
│   ├── Cargo.lock
│   ├── README.md
│   └── src\
│       └── lib.rs
│
├── chronos-execution-runtime\            # EXECUTION — Execution runtime
│   ├── Cargo.toml
│   ├── Cargo.lock
│   ├── README.md
│   └── src\
│       └── lib.rs
│
│
├── chronos-runtime-loop\                 # RUNTIME — Continuous 6-phase tick engine
│   ├── Cargo.toml
│   ├── Cargo.lock
│   ├── README.md
│   └── src\
│       └── lib.rs                         # ContinuousRuntimeLoopEngine,
│                                          # execute_tick(), RuntimeMode (Live/Replay),
│                                          # InternalMetrics
│                                          # Phase 1: Ingestion
│                                          # Phase 2: Cognitive Update
│                                          # Phase 3: Decision
│                                          # Phase 4/5: Execution
│                                          # Phase 6: Feedback
│
├── chronos-runtime-stability\            # RUNTIME — Stability guard
│   ├── Cargo.toml
│   ├── Cargo.lock
│   ├── README.md
│   └── src\
│       └── lib.rs
│
├── chronos-system-integrity\             # RUNTIME — Integrity monitoring
│   ├── Cargo.toml
│   ├── Cargo.lock
│   ├── README.md
│   └── src\
│       └── lib.rs
│
├── chronos-decision-orchestrator\        # RUNTIME — Decision orchestration layer
│   ├── Cargo.toml
│   ├── Cargo.lock
│   ├── README.md
│   └── src\
│       └── lib.rs
│
├── chronos-event-orchestrator\           # RUNTIME — Event routing
│   ├── Cargo.toml
│   ├── Cargo.lock
│   ├── README.md
│   └── src\
│       └── lib.rs                         # EventOrchestrator
│
│
├── chronos-adapter-clipboard\            # PERCEPTION — Clipboard adapter
│   ├── Cargo.toml
│   ├── Cargo.lock
│   ├── README.md
│   └── src\
│       └── lib.rs
│
├── chronos-adapter-filewatcher\          # PERCEPTION — File system watcher
│   ├── Cargo.toml
│   ├── Cargo.lock
│   ├── README.md
│   └── src\
│       └── lib.rs
│
├── chronos-adapter-git\                  # PERCEPTION — Git activity adapter
│   ├── Cargo.toml
│   ├── Cargo.lock
│   ├── README.md
│   └── src\
│       └── lib.rs
│
├── chronos-adapter-window-focus\         # PERCEPTION — Window focus adapter
│   ├── Cargo.toml
│   ├── Cargo.lock
│   ├── README.md
│   └── src\
│       └── lib.rs
│
│
├── chronos-api-bridge\                   # API — HTTP + SSE server (Axum)
│   ├── Cargo.toml
│   ├── Cargo.lock
│   └── src\
│       ├── lib.rs                         # Crate root / module declarations
│       ├── handlers.rs                    # All route handler functions (25 KB)
│       ├── router.rs                      # Route table + BridgeState assembly
│       └── state.rs                       # BridgeState struct
│                                          #   Routes:
│                                          #   GET  /api/health
│                                          #   GET  /api/events/stream
│                                          #   GET  /api/state
│                                          #   GET  /api/session/current
│                                          #   GET  /api/reasoning/forecasts
│                                          #   GET  /api/reasoning/diagnostics
│                                          #   GET  /api/execution/commitments/active
│                                          #   POST /api/execution/generate-recovery-plan
│                                          #   POST /api/execution/restore-workspace
│                                          #   POST /api/decision/simulate
│                                          #   POST /api/perception/ingest
│
├── chronos-telemetry-bridge\             # API — Telemetry ingestion (browser/VSCode)
│   ├── Cargo.toml
│   ├── Cargo.lock
│   └── src\
│       ├── lib.rs
│       ├── browser.rs
│       ├── manual.rs
│       └── vscode.rs
│
│
├── chronos-daemon\                       # DAEMON — Main system process (wires all crates)
│   ├── Cargo.toml
│   ├── Cargo.lock
│   └── src\
│       └── main.rs                        # 37 KB — primary wiring entrypoint
│
│
├── ⚠️ chronos-commitment-inference-engine\  # STUB — NO SOURCE CODE
│   └── Cargo.toml                         # Only file. src/ does NOT exist.
│                                          # Deps declared: chronos-core, chronos-bus
│                                          # Status: UNIMPLEMENTED
│
│
├── ── TAURI DESKTOP APP ───────────────────────────────────────────────
│
├── src-tauri\                             # Tauri desktop shell
│   ├── Cargo.toml
│   ├── Cargo.lock                         # 170 KB
│   ├── build.rs
│   ├── tauri.conf.json
│   ├── .cargo\
│   │   └── config.toml
│   ├── icons\
│   │   └── icon.ico
│   └── src\
│       ├── main.rs                        # App entry point (7.6 KB)
│       ├── server.rs                      # ★ LARGEST FILE — 151 KB monolith HTTP server
│       ├── db.rs                          # SQLite DB layer (12.7 KB)
│       ├── che.rs                         # Cognitive heuristic engine (3.6 KB)
│       ├── consequence.rs                 # Consequence model (1.9 KB)
│       ├── watcher.rs                     # File watcher (3.5 KB)
│       └── window_focus.rs               # Window focus tracking (2.4 KB)
│
│
├── ── FRONTENDS ───────────────────────────────────────────────────────
│
├── chronos-pilot\                         # ★ ACTIVE FRONTEND — React cognitive dashboard
│   ├── .env.example
│   ├── .gitignore
│   ├── index.html
│   ├── package.json
│   ├── package-lock.json
│   ├── tsconfig.json
│   ├── vite.config.ts
│   ├── README.md
│   ├── server.ts                          # Local dev proxy server (17.7 KB)
│   ├── chronos_local_db.json             # Cached local data
│   ├── metadata.json
│   └── src\
│       ├── App.tsx                        # Main app component (41.7 KB)
│       ├── index.css
│       ├── main.tsx
│       ├── types.ts
│       ├── data\
│       │   └── mockDB.ts                  # Mock data (20.5 KB)
│       └── components\
│           ├── ARCPanel.tsx               # ARC panel (4.5 KB)
│           ├── CommitmentList.tsx         # Commitment display (11.4 KB)
│           ├── DatabaseViewer.tsx         # DB viewer (6.9 KB)
│           ├── InteractiveSandbox.tsx     # Interactive sandbox (13.4 KB)
│           ├── RiskForecaster.tsx         # Risk forecasting UI (8.4 KB)
│           └── WorkspaceRestorer.tsx      # Workspace restore UI (8.4 KB)
│
├── ui\                                    # LEGACY FRONTEND — Tauri React UI
│   ├── .gitignore
│   ├── index.html
│   ├── package.json
│   ├── package-lock.json
│   ├── tsconfig.json
│   ├── vite.config.ts
│   ├── README.md
│   ├── public\
│   │   ├── favicon.svg
│   │   └── icons.svg
│   ├── dist\                              # Built bundle
│   │   ├── index.html
│   │   ├── favicon.svg
│   │   ├── icons.svg
│   │   └── assets\
│   │       ├── index-BJpFdwuy.css
│   │       └── index-BkOAGRK7.js
│   └── src\
│       ├── App.tsx                        # Legacy main app (37.8 KB)
│       ├── config.ts
│       ├── index.css
│       ├── main.tsx
│       ├── types.ts
│       ├── data\
│       │   └── mockDB.ts
│       └── components\
│           ├── ARCPanel.tsx
│           ├── CommitmentList.tsx
│           ├── DatabaseViewer.tsx
│           ├── FlightRecorderPanel.tsx    # ★ LARGEST UI FILE — 57 KB
│           ├── InteractiveSandbox.tsx
│           ├── RiskForecaster.tsx
│           └── WorkspaceRestorer.tsx
│
│
├── ── BROWSER & EDITOR EXTENSIONS ────────────────────────────────────
│
├── extensions\
│   ├── browser-extension\
│   │   ├── manifest.json
│   │   └── background.js                  # Browser extension background script
│   └── vscode-connector\
│       ├── package.json
│       ├── package-lock.json
│       └── extension.js                   # VSCode extension connector
│
│
├── ── PYTHON WORKER ───────────────────────────────────────────────────
│
└── python-worker\
    ├── requirements.txt
    ├── worker.py                           # ★ Main worker — 31 KB
    ├── arc_crawler.py                      # ARC browser crawler
    ├── cde_parser.py                       # CDE event parser
    ├── embeddings.py                       # Embedding generation
    ├── simulator_worker.py                 # Simulation worker
    └── audit_results.json                  # Audit output data
```

---

## CRATE DEPENDENCY SUMMARY

```
chronos-daemon  ──depends on──► chronos-api-bridge
                                chronos-runtime-loop
                                chronos-bus
                                chronos-store-sqlite
                                chronos-telemetry-bridge
                                chronos-adapter-*
                                (+ all reasoning/execution crates)

chronos-runtime-loop  ──────►  chronos-core
                                chronos-bus
                                chronos-store
                                chronos-logging
                                chronos-reasoning-commitments
                                chronos-reasoning-intent
                                chronos-reasoning-continuity
                                chronos-reasoning-coherence
                                chronos-reasoning-decision
                                chronos-execution-orchestration
                                chronos-execution-feedback

chronos-api-bridge  ─────────► chronos-core
                                chronos-bus
                                chronos-store-sqlite
                                chronos-registry
                                chronos-reasoning-commitments
                                chronos-memory-entity-resolution
                                chronos-memory-sessions
                                chronos-event-orchestrator

chronos-reasoning-commitments ► chronos-core
                                chronos-memory-entity-resolution
                                chronos-memory-sessions
                                chronos-memory-state

chronos-bus  ────────────────► chronos-core

chronos-core  ───────────────► (no internal deps — root primitive)
```

---

## FILE SIZE LANDMARKS

| File | Size | Note |
|---|---|---|
| `src-tauri/src/server.rs` | 151 KB | Largest file in repo |
| `ui/src/components/FlightRecorderPanel.tsx` | 57 KB | Largest UI component |
| `chronos-daemon/src/main.rs` | 37 KB | Main system wiring |
| `python-worker/worker.py` | 31 KB | Python core worker |
| `chronos-reasoning-commitments/src/lib.rs` | 23 KB | Full commitment engine |
| `chronos-memory-entity-resolution/src/lib.rs` | 22 KB | Entity resolution |
| `chronos-pilot/src/App.tsx` | 42 KB | Active frontend root |

---

## STATUS FLAGS

| Status | Meaning |
|---|---|
| ✅ Implemented | Has source code, compiles, integrated into runtime |
| ⚠️ Stub | Cargo.toml exists but no `src/` directory |
| 🔴 Not integrated | Exists but not wired into any other crate |

| Crate | Status |
|---|---|
| All 38 other crates | ✅ Implemented |
| `chronos-commitment-inference-engine` | ⚠️ Stub — `src/lib.rs` does not exist |
