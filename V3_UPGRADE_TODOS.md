# Phase 2: Chronos Pilot v3.0 Upgrade TODOs

This document outlines the structured action items required to upgrade the current MVP (Phase 1) architecture to the finalized v3.0 System Requirements Specification (SRS).

### 1. Core Architecture & Infrastructure Hardening
- [ ] **Dynamic Sidecar Security:** Replace statically configured Python sidecar startup with **Stdin Token Delivery** (passing `X-Chronos-Auth-Token` via `Stdio::piped()`) and **Dynamic Port Negotiation** (ephemeral ports).
- [ ] **Executable Hash Validation:** Implement a startup check in the Rust daemon that calculates the SHA-256 hash of each Python sidecar binary and verifies it against a signed manifest.
- [ ] **Database Encryption:** Migrate the standard `rusqlite` implementation to use **SQLCipher** for 256-bit AES database encryption at rest, retrieving the passphrase from the host OS keyring.
- [ ] **Vector & FTS Search Subsystem:** Fully implement the `sqlite-vec` (384 dimensions) and `FTS5` virtual tables (`context_embeddings`, `context_fts`) to enable localized semantic and keyword querying.
- [ ] **Vector Migration Protocol:** Implement an atomic swap protocol (`context_embeddings_new`) for upgrading local embedding models without database locks.

### 2. Data Model & Schema Expansion (SQLite Supergraph)
- [ ] **Inherited Context Nodes:** Expand `context_nodes` by implementing specialized child extension tables: `file_nodes`, `message_nodes`, `terminal_nodes`, and `url_nodes`.
- [ ] **Project Supergraph Containerization:** Implement the `project_community_map` table to allow a single user-facing Project to encapsulate multiple semantic communities.
- [ ] **Entity Resolution Layer (ERL):** Add `canonical_entities`, `entity_merge_history`, and `entity_split_history` to track consolidated variations of names.
- [ ] **Audit & Provenance Tables:** Create `source_provenance`, `adapter_fingerprints` (tracking DOM layout stability), and `project_membership_history` (tracking node community shifts).

### 3. Passive Ingestion & Privacy Isolation Layer
- [ ] **Decoupled Ingestion Adapters & Fingerprinting:** Implement the `IIngestionAdapter` TypeScript interface (WhatsApp, AI Chats, GenericDOM) and DOM Fingerprint Versioning to detect and fallback when web layouts change.
- [ ] **Automated Document Parsing:** Integrate an automated multi-column PDF/DOCX parser and a local lightweight OCR engine for images dropped into watched directories.
- [ ] **Workspace Sensitivity Classifier (WSC):** Implement a pre-write redaction pipeline (Regex for credentials, ONNX NER for PII redaction).
- [ ] **Focus Tracker Blocklist:** Hardcode logic to pause active window telemetry when sensitive apps or secure browser domains are in focus.
- [ ] **Vector Differential Privacy:** Implement $\epsilon$-DP by injecting low-magnitude Gaussian noise into embeddings to prevent vector inversion attacks.

### 4. Memory Engine & Project Intelligence
- [ ] **O(N log N) Semantic Routing:** Implement a candidate generation filter using `sqlite-vec` ANN to query only the top 50 semantic neighbors when a new node is ingested.
- [ ] **Unassigned Commitment Inbox:** Calculate a Project Match Score (MS). If MS < 0.80, route the ingested item to a null-commitment inbox rather than forcing a cluster assignment.
- [ ] **In-Memory `igraph` Sidecar Projection:** Implement the local Python `graph_worker` sidecar using C-core `igraph` to execute semantic Leiden Clustering.
- [ ] **Decision Conflict Detection (DCD):** Create a background job to scan same-project nodes for contradictions, logging findings into the `decision_conflicts` table.
- [ ] **Cross-Platform Thread Association:** Link disjointed nodes automatically if cosine similarity $\ge 0.78$.

### 5. AI Extraction Engines (ODE & DDE)
- [ ] **Opportunity Detection Engine (ODE):** Scan passive feeds to calculate classification scores and trigger a HUD prompt ("New Project Opportunity Detected").
- [ ] **Deadline & Commitment Discovery Engine (DDE):** Build an NLP heuristic pipeline to extract implicit deadlines and deliverables into `deadline_candidates` and `deliverable_candidates` tables.

### 6. Cognitive Capacity, Workload & Scheduling (PCM, DPE, SDE)
- [ ] **Personal Capacity Model (PCM) & Velocity Tracking:** Track active editor focus events and interruption frequency to build an empirical availability profile.
- [ ] **Deliverability Prediction Engine (DPE) Refinement:** Implement the formal mathematical sigmoid logistic calculation for $P(success)$ based on integrated focus hours and a nightly Brier Score audit loop.
- [ ] **Schedule Drift Engine (SDE):** Implement the Drift Metric to automatically shift unstarted task start dates and defer low-priority tasks if a schedule collapses due to user absence.
- [ ] **Nightly Parameter Recalibration:** Build an optimization loop that adjusts PCM weights based on historical user adjustments.

### 7. Workspace Connectors & The Context Continuation Engine (CCE)
- [ ] **Workspace Connector Framework:** Build local WebSocket/REST hooks (`IWorkspaceConnector`) for IDEs (VSCode, Cursor, Windsurf).
- [ ] **CCE Mode A (Passive Assistance):** Trigger background staging for gap logs when the user is idle $\ge 20$ minutes.
- [ ] **CCE Mode B (Explicit Continuation):** Implement rich `project_snapshots` and auto-generate a `reconstruction_narrative.md` when triggering "Continue Project".
- [ ] **Autonomous Research Continuation (ARC):** Implement the sandboxed background agent with Staged Format Isolation and Provenance Comment Staining.
- [ ] **Three-Tier Hybrid Memory Decay:** Implement Hot, Warm, and Archived memory states based on relevance decay calculations.

### 8. Interactive HUD & Tracing (Frontend)
- [ ] **Layer 1 & 2 Refinements:** Implement System Notification Toasts and "Deliverability Warning Cards".
- [ ] **Layer 3 (Command Center Overlay):** Implement the global hotkey modal (Ctrl+Shift+Space) containing the **Action Consent Gate**.
- [ ] **"Take Me Back" Traceability Trigger:** Implement HUD buttons next to nodes that directly open the captured session hash URL, local file, or WhatsApp thread.
- [ ] **Work Abandonment Alerting:** Continuously evaluate tracked items and fire Toasts if a task enters the `ABANDONED_SUSPECT` status.
