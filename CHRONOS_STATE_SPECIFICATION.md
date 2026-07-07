# Chronos State Specification

## 1. Executive Summary
The Event Layer (Layer 1) records reality. The Memory/Session Layer (Layer 2) organizes reality. The **Knowledge Layer (Layer 2 - Entity Resolution)** transforms reality into state.

`ChronosState` is the canonical runtime representation of understood reality within the Personal Context Operating System (PCOS). It acts as the synthesized cognitive world model of the user at any given microsecond. Every Reasoning Engine, Decision Engine, Execution Engine, and UI component consumes `ChronosState` as its foundational source of truth.

This document establishes the canonical knowledge contract defining `ChronosState`, its boundaries, its ownership, and its relationships across the PCOS architecture.

---

## 2. Canonical Definition

### 2.1 Core Attributes
*   **Identity:** A universally unique identifier (UUID) coupled with an immutable monotonic version number representing a specific snapshot of the user's cognitive world model.
*   **Lifecycle:** Re-evaluated continuously upon the arrival of new semantic interpretations (e.g., a closed `CognitiveSession` or a resolved `Commitment`). It never "dies", it only advances in version.
*   **Ownership:** Completely owned and maintained by the **Memory/Knowledge Layer (Layer 2)**. Higher layers (Reasoning, Decision) read this state but cannot directly mutate it without dispatching an event to the bus.
*   **Provenance:** Derived exclusively from the Knowledge Graph, which is in turn derived from `CognitiveSession`s and `ChronosEvent`s. It is a projection of the graph.
*   **Freshness:** Timestamped. Any consumer evaluating a `ChronosState` can calculate its age to determine if it requires a manual sync or refresh before making critical decisions.
*   **Confidence:** An aggregated probabilistic score (0.0 to 1.0) indicating how certain the Knowledge Layer is that this snapshot accurately reflects the user's actual reality.
*   **Relationships:** 1:1 mapping with the global Knowledge Graph at a specific temporal moment. 1:N mapping to active `Project`s and `Commitment`s.

---

## 3. Operational Profile

*   **Represents:** What Chronos knows to be true about the user's current goals, context, and focus right now.
*   **Consumes:** Aggregated outputs from `CognitiveSession`s, Entity Resolution loops, and Knowledge Graph mutations.
*   **Produces:** The foundational contract struct (`ChronosState`) broadcasted on the Cognitive Bus for consumption by Layer 3, Layer 4, and Layer 5.
*   **Dependencies:** Relies entirely on the stability of the Event Store and the Memory Layer's graph resolution.
*   **Failure Modes:**
    *   *Stale State:* High event latency causes `ChronosState` to reflect a world model from several minutes ago.
    *   *Hallucinated State:* Over-aggressive Entity Resolution incorrectly merges two distinct projects, polluting the global state.
*   **Persistence Requirements:** The abstract *schema* of the state is not persisted, but the *Knowledge Graph* it projects from is completely durable. `ChronosState` is simply a materialized view of the graph.
*   **Replay Requirements:** Because `ChronosState` is a deterministic projection of `ChronosEvent`s through `CognitiveSession`s, replaying the Event Store through the Memory Layer must yield the exact same series of `ChronosState` snapshots.
*   **Explainability Requirements:** Every active element in a `ChronosState` must contain a pointer (`provenance_id`) tracing back to the `CognitiveSession` or `ChronosEvent` that justified its inclusion in the current world model.

---

## 4. State Incorporation

`ChronosState` is an aggregation of multiple structural domains:

*   **Cognitive Sessions:** Tracks which session is currently active (in flow) and which are recently decayed.
*   **Projects:** Tracks the overarching structural goals currently dominating the user's attention.
*   **Commitments:** Lists the specific, actionable promises the user has implicitly or explicitly made that require execution.
*   **Artifacts:** Tracks the abstract entities (e.g., "The Final Report") currently loaded into cognitive working memory.
*   **Files:** Maps Artifacts to physical host-OS coordinates (e.g., `/docs/final_report.pdf`).
*   **Conversations:** Tracks semantic threads of communication (Slack, Email) impacting current Commitments.
*   **Decisions:** Includes the ledger of recent `ChronosDecision`s made by the orchestrator (to prevent duplicate interventions).
*   **Actions:** Includes active `ChronosAction`s currently being executed by Layer 5 (Execution).
*   **Context:** Contains the user's inferred cognitive capacity (e.g., Focus Intensity, Burnout Risk) passed up from the Reasoning Layer.

---

## 5. State Classifications

Within the global `ChronosState`, individual sub-components (like a Project or a Session) are classified to aid Reasoning Engines:

*   **Active State:** The entity is actively being manipulated or was manipulated within the current context decay window. (e.g., The file the user is currently typing in).
*   **Dormant State:** The entity remains unresolved (e.g., a pending Commitment) but has slipped outside the active decay window. It is ripe for Intervention analysis.
*   **Archived State:** The entity is resolved, completed, or intentionally abandoned. It is retained in the Knowledge Graph for historical velocity calculations but filtered out of the active `ChronosState` payload to reduce overhead.
*   **Conflicting State:** The Memory Layer has detected contradictory evidence (e.g., a file was deleted locally, but a recent `ChronosEvent` suggests the user is searching for it). Flags the Reasoning Layer to synthesize a clarification prompt.
*   **Incomplete State:** An entity exists but lacks critical properties (e.g., an Artifact was created but not yet assigned to a Project).

---

## 6. Architectural Relationships

How `ChronosState` maps to the Chronos Object Model (COM):

*   **ChronosEvent:** The atomic building block. Events mutate the Knowledge Graph, which in turn triggers a new `ChronosState` version.
*   **CognitiveSession:** `ChronosState` includes a pointer to the currently active or most recently closed `CognitiveSession`.
*   **ChronosIntent:** A user intent directly alters the `ChronosState` by introducing a new active Commitment or shifting Project focus.
*   **ChronosDecision:** The Decision Orchestrator (Layer 4) consumes `ChronosState` to evaluate Intervention math (e.g., Silence Cost vs Interruption Cost).
*   **ChronosAction:** Layer 5 consumes `ChronosState` to know *how* to execute an action (e.g., querying the state for the physical file paths needed to perform Workspace Restoration).
*   **Knowledge Graph Nodes:** `ChronosState` is a point-in-time, hierarchical serialization of the most active/relevant Knowledge Graph Nodes.
*   **Timeline Entries:** While Timeline Entries represent history, `ChronosState` represents the bleeding edge of the *present*.

---

## 7. UI Ownership and Consumption (Layer 6)

The UI (Layer 6) consumes data through two distinct pathways to maintain strict architectural boundaries:

**Direct Consumption of `ChronosState` (Memory/Knowledge Layer):**
*   *Workspace Restoration (Quadrant 04):* Consumes `ChronosState` directly to list dormant projects and their physical file coordinates.
*   *Timeline View:* Consumes `ChronosState` (and historical states) to render the block visualization of reality.

**Consumption of Derived Outputs (Reasoning/Decision Layers):**
*   *Risk Analysis (Quadrant 01):* Does **not** calculate risk from `ChronosState`. It consumes pre-calculated forecasts and Monte Carlo probabilities emitted by the Reasoning Layer (Layer 3).
*   *Strategy Ledger (Quadrant 02):* Consumes execution plans synthesized by the Execution Layer (Layer 5), not raw state.
*   *Theory Simulator / Interventions (Mode 03):* Consumes `ChronosDecision`s broadcast by the Decision Orchestrator (Layer 4).

By enforcing this split, the UI remains a pure projection layer, never attempting to calculate predictions or make decisions based on raw `ChronosState`.
