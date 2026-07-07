# IMPLEMENTATION_STATUS.md
*Authoritative Repository-Wide Feature Implementation Tracker*

---

## 1. Feature Implementation Matrix

| Subsystem Component | Target Layer | current Status | Code Evidence |
| :--- | :--- | :--- | :--- |
| **COM Schemas** | Layer 0 (Infra) | **Complete** | Struct definitions in `chronos-core/src/lib.rs`. |
| **Cognitive EventBus** | Layer 0 (Infra) | **Complete** | Broadcast implementation in `chronos-bus/src/lib.rs`. |
| **Durable SQLite Store** | Layer 0 (Infra) | **Complete** | Store implementation in `chronos-store-sqlite/src/lib.rs`. |
| **Window Focus Observer** | Layer 1 (Percept) | **Complete** | Win32 hooks in `chronos-adapter-window-focus/src/lib.rs`. |
| **Clipboard Observer** | Layer 1 (Percept) | **Complete** | Copy listener in `chronos-adapter-clipboard/src/lib.rs`. |
| **Git Log Observer** | Layer 1 (Percept) | **Complete** | Scraper tasks in `chronos-adapter-git/src/lib.rs`. |
| **Entity Resolution Engine** | Layer 2 (Memory) | **Functional** | Heuristic matching in `chronos-memory-entity-resolution/src/lib.rs`. |
| **Cognitive Session Engine** | Layer 2 (Memory) | **Functional** | Decay algorithm in `chronos-memory-sessions/src/lib.rs`. |
| **State Projector Engine** | Layer 2 (Memory) | **Functional** | Projection model in `chronos-memory-state/src/lib.rs`. |
| **Reflection Engine** | Layer 3 (Reason) | **Functional** | Heuristic interpretation in `chronos-reasoning-reflection/src/lib.rs`. |
| **Commitment Discovery Engine** | Layer 3 (Reason) | **Functional** | Evidence scanning in `chronos-reasoning-commitments/src/lib.rs`. |
| **Deadline Discovery Engine (DDE)**| Layer 3 (Reason) | **Functional** | Regex extraction in `chronos-reasoning-dde/src/lib.rs`. |
| **Personal Capacity Model (PCM)** | Layer 3 (Reason) | **Functional** | Metrics calculation in `chronos-reasoning-pcm/src/lib.rs`. |
| **Risk Forecast Engine** | Layer 3 (Reason) | **Functional** | Logistic decay modeling in `chronos-reasoning-risk/src/lib.rs`. |
| **Decision Orchestrator** | Layer 4 (Decision) | **Functional** | Intervention evaluation in `chronos-decision-orchestrator/src/lib.rs`. |
| **Context Continuation Engine (CCE)**| Layer 5 (Exec) | **Functional** | Checklist generation in `chronos-execution-cce/src/lib.rs`. |
| **Execution Runtime** | Layer 5 (Exec) | **Functional** | Workspace restoration tasks in `chronos-execution-runtime/src/lib.rs`. |
| **API Bridge Server** | Layer 6 (Interact) | **Complete** | HTTP endpoints and SSE streams in `chronos-api-bridge/src/router.rs`. |
| **Cockpit Mission Control UI** | Layer 6 (Interact) | **Complete** | React modules inside `chronos-pilot/src/`. |
| **Commitment Inference Crate** | Layer 3 (Reason) | **Stub** | Crate contains only a `Cargo.toml`; no source code exists. |
| **Vector Similarity Engine** | Layer 0 (Infra) | **Partial** | Fallback to text vector mapping if `vec0` load fails (*Evidence: `src-tauri/src/db.rs#L191`*). |
| **SentenceTransformer Embeddings** | Sidecar | **Partial** | Fallback mock hashes generated if dependency missing (*Evidence: `python-worker/embeddings.py#L27`*). |

---

## 2. Incomplete & Stub Feature References

*   **`chronos-commitment-inference-engine`**:
    *   *Maturity*: **Stub**.
    *   *Evidence*: The crate folder contains only a `Cargo.toml` file with no `src/` directory (*Evidence: `PROJECT_STRUCTURE.md`*).
*   **Vector Database Fallback**:
    *   *Maturity*: **Partial**.
    *   *Evidence*: Fallback table `context_embeddings` is used if SQLite `vec0` load fails, storing vector representations as raw strings (*Evidence: `src-tauri/src/db.rs#L191`*).

---
