# Chronos Object Model (COM)

*This document defines the canonical object model for the Chronos Personal Context Operating System. It establishes the strict contracts, ownership boundaries, lifecycles, and relationships for every object in the system. No subsystem may invent its own data structures for core runtime operations; everything must conform to the COM.*

---

## 1. The Canonical Objects

### 1.1 `ChronosEvent`
*The fundamental unit of perception. The raw recording of reality.*
- **Owner:** Perception Layer
- **Created by:** Adapters (e.g., Git, File System, Clipboard)
- **Modified by:** None
- **Immutable:** Yes
- **Persisted:** Yes (in the immutable event log, potentially truncated after memory consolidation)
- **Replayable:** Yes
- **Hashable:** Yes
- **Signed:** No
- **Versioned:** Yes (Schema versions for backward compatibility)
- **Lifetime:** Forever in event log (or until aggressively archived)

### 1.2 `ChronosState`
*The aggregated world model representing the ground truth at any millisecond.*
- **Owner:** Memory Layer
- **Created by:** Event Processors / State Reducers
- **Modified by:** Knowledge Mutation Pipeline
- **Immutable:** No (Represents the active snapshot)
- **Persisted:** Checkpointed periodically
- **Replayable:** Derived entirely from `ChronosEvent` replay
- **Versioned:** Yes (Checkpoints)
- **Lifetime:** Exists dynamically in memory; persisted via snapshots

### 1.3 `ChronosIntent`
*The inferred objective the user is pursuing. The center of the semantic graph.*
There are two distinct classifications of Intent:
- **Goal Intent:** Long-lived, overarching objectives (*e.g., Build Chronos, Publish a paper*).
- **Session Intent:** Ephemeral, immediate objectives bound to a Cognitive Session (*e.g., Fix the graph bug, Read chapter 6*). Multiple Session Intents contribute to a Goal Intent.
- **Owner:** Intent Engine (Reasoning Layer)
- **Created by:** Inference (Pattern Discovery / DDE)
- **Updated by:** Reasoning Layer
- **Confidence:** Required (e.g., 85%)
- **Parent Concepts:** Required
- **Current Status:** Required (Active, Paused, Completed, Abandoned)
- **Persistence:** Always
- **Version History:** Yes (Tracks how the system's understanding of the intent evolved)

### 1.4 `ChronosDecision`
*A formal resolution to intervene or modify the system.*
- **Owner:** Decision Orchestrator
- **Created by:** Decision Orchestrator
- **Modified by:** None (Decisions are final upon creation, though they can expire)
- **Immutable:** Yes
- **Persisted:** Yes (Decision Ledger)
- **Can be revoked:** Yes
- **Can expire:** Yes (Time-to-Live is required)
- **Can generate actions:** Yes
- **Confidence:** Required
- **Explanation:** Required (Human-readable "Why")
- **Evidence:** Required (Pointers to the Predictions and Intents that supported it)

### 1.5 `ChronosAction`
*An executable command translating a Decision into a physical change.*
- **Owner:** Execution Layer
- **Created by:** Action Executor (from a Decision)
- **Immutable:** Yes
- **Persisted:** Yes (Audit log)
- **State:** Tracks execution lifecycle (Pending, Authorized, Executing, Success, Failed)
- **Reversible:** Must implement a rollback method where physically possible.

### 1.6 `ChronosReflection`
*The learning record generated after an Action or Decision concludes.*
- **Owner:** Reflection Engine
- **Created by:** Reflection Engine
- **Immutable:** Yes
- **Persisted:** Yes
- **Purpose:** Used to adjust Personal Capacity Model (PCM) weights, Confidence thresholds, and Prompt selections.

### 1.7 `ChronosCapability`
*A first-class object representing what the system is currently able to do. Prevents subsystems from blindly assuming functionality.*
- **Owner:** Plugin Substrate / Infrastructure
- **Created by:** Adapter / Plugin Registration
- **Updated by:** Health Checks
- **Properties:**
  - **Status:** Enabled / Disabled
  - **Health:** Healthy / Degraded / Failing
  - **Permission:** Granted / Denied (Consent gates)
  - **Confidence/Quota:** e.g., 100%, or Remaining Tokens, Latency ms.
- *Every engine must query `ChronosCapability` before planning an Action.*

---

## 2. The Ontology (Relationships)

The objects defined above interact in a strict, unidirectional semantic flow. This ontology defines how meaning is constructed from reality:

```text
ChronosEvent
     ↓ (creates)
Observation
     ↓ (produces)
Artifact
     ↓ (supports)
Concept
     ↓ (infers)
Intent
     ↓ (generates)
Commitment
     ↓ (feeds)
Prediction
     ↓ (supports)
Decision
     ↓ (triggers)
Action
     ↓ (creates)
Reflection
```

---

## 3. The Implementation Roadmap

To prevent architectural churn, Chronos will be built according to the following phased roadmap, prioritizing stable contracts over immediate visible features.

| Phase | Build | Why First? |
| :--- | :--- | :--- |
| **0** | **Chronos Object Model (COM)** | Defines every runtime object and ownership contract. (This document). |
| **1** | **Ontology & Semantic Hierarchy** | Establishes the meaning and relationships of everything Chronos knows. |
| **2** | **Chronos Core Protocol (CCP)** | Specifies how objects move through the runtime lifecycle. |
| **3** | **Persistence Layer** | Maps the object model into SQLite without inventing concepts during implementation. |
| **4** | **Cognitive Bus** | Moves well-defined objects between components. |
| **5** | **Perception Adapters** | Begin generating `ChronosEvents` from Git, Clipboard, and UI. |
| **6** | **Knowledge Graph & Memory** | Transform events into durable knowledge via the Semantic Hierarchy. |
| **7** | **Reasoning Engines** | Add inference, prediction, and Intent generation (Goal vs Session). |
| **8** | **Decision Orchestrator** | Turn evidence into coherent Decisions (with Confidence, Evidence, Explanation). |
| **9** | **Execution & Interaction** | CCE, ARC, HUD, Timeline, and user-facing workflows. |
