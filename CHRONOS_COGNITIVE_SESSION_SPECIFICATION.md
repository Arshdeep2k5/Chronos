# Chronos Cognitive Session Specification

## 1. Executive Summary
Chronos does not merely record events; it preserves cognitive continuity. The **Cognitive Session** is the foundational primitive that bridges the gap between Raw Observation (Layer 1) and Meaningful Continuity (Layer 5). 

While raw `ChronosEvent` payloads provide a stateless history of what occurred, a `CognitiveSession` groups these events into a contiguous, semantic block of human focus. It represents a continuous period of intent-driven work, providing the context necessary for the operating system to predict decay, forecast risk, and restore flow.

This document establishes the canonical definition and lifecycle of a Cognitive Session within the Personal Context Operating System (PCOS).

---

## 2. Canonical Definition

### 2.1 Core Attributes
*   **Identity:** A universally unique identifier (UUID) identifying a specific block of focused human cognition.
*   **Lifecycle:** Begins upon the detection of intent-aligned activity, persists through active observation, and concludes upon explicit cessation (e.g., locking the screen) or implicit decay (timeout of relevant activity).
*   **Boundaries:** Bounded temporally by start/end timestamps and bounded semantically by the set of projects and artifacts being manipulated.
*   **Provenance:** Derived exclusively from an immutable ledger of `ChronosEvent`s. A session is an abstraction *over* reality, not a replacement for it.
*   **Confidence:** Contains a probabilistic score (0.0 to 1.0) indicating how certain the system is that the grouped events truly belong to a single cognitive context.
*   **Relationships:** N:1 mapping of `ChronosEvent`s to a `CognitiveSession`. N:M mapping of `CognitiveSession`s to `Project`s and `Artifact`s.
*   **Ownership:** Governed and maintained by **Layer 2 (Memory)**, with boundaries heuristically determined by **Layer 3 (Reasoning)**.

---

## 3. Operational Profile

*   **Represents:** A contiguous, uninterrupted block of human attention directed toward a specific contextual goal.
*   **Consumes:** A stream of raw `ChronosEvent`s from the Cognitive Bus.
*   **Produces:** Aggregated semantic summaries (e.g., "Worked on Project X for 2 hours, modifying 4 files") broadcasted back to the bus as `ChronosState` updates.
*   **Dependencies:** Relies on the Event Store (for historical event retrieval) and the Memory Layer (for semantic resolution).
*   **Failure Modes:** 
    *   *Over-grouping:* Merging distinct tasks into one session due to rapid context switching.
    *   *Fragmentation:* Splitting a continuous workflow into multiple sessions due to brief pauses (e.g., thinking, reading physical notes).
*   **Persistence Requirements:** Persisted in the Knowledge Graph (Layer 2) as a first-class node linking events to concepts. Must survive system reboots.
*   **Replay Requirements:** Must be fully reconstructible from the raw `ChronosEvent` ledger. If the session node is deleted, re-running the reasoning engine over the Event Store must yield an identical (or improved) session boundary.
*   **Explainability Requirements:** Every session must be able to justify its boundaries by pointing to the exact `ChronosEvent`s (e.g., "Session started because VSCode was opened; Session ended because no keyboard input was detected for 15 minutes").

---

## 4. Session Mechanics

### 4.1 Boundary Detection
*   **Session Start Detection:** Triggered when the Reasoning Layer detects a density of contextual `ChronosEvent`s emerging from a baseline of silence, or an explicit context switch (e.g., moving from a YouTube tab to an IDE).
*   **Session End Detection:** Triggered by explicit halts (Sleep, Screen Lock, App Quit) or implicit decay (time elapsed since last contextual `ChronosEvent` exceeds a dynamic threshold).
*   **Session Continuation:** Brief interruptions (e.g., a 2-minute Slack reply) do not end a session. The Reasoning Layer bridges these gaps if the user returns to the primary artifacts immediately.
*   **Session Resurrection:** When a user resumes a dormant session, a *new* Cognitive Session is created, but it explicitly holds a `resumes_session_id` pointer to the historical session, forming a continuity chain.

### 4.2 Graph Mutations
*   **Session Merging:** If retroactive reasoning determines that two adjacent sessions were actually the same continuous workflow (e.g., separated by an unlogged offline reading break), they are merged, and the Confidence Score is updated.
*   **Session Splitting:** If deep analysis reveals a user was multi-tasking on two entirely unrelated projects simultaneously, a single session may be retroactively split into two concurrent sessions.

### 4.3 Evaluation
*   **Session Confidence Scoring:** Calculated based on event density, artifact relation (do the modified files belong to the same project?), and app cohesion.
*   **Session Attribution:** The process of mapping the Cognitive Session to a specific `Commitment` or `Project` in the Knowledge Graph.

---

## 5. Architectural Relationships

A Cognitive Session interacts with the Chronos Object Model (COM) as follows:

*   **ChronosEvent:** A Session is a container for N `ChronosEvent`s. Events are the atomic evidence; the Session is the macroscopic theory.
*   **ChronosIntent:** A Session is the physical manifestation of an Intent. 
*   **ChronosDecision & ChronosAction:** The Decision Layer evaluates the decay of past Sessions to decide when to trigger a `ChronosAction` (e.g., an intervention alert).
*   **ChronosState:** The active Cognitive Session defines the current `ChronosState` of the user.
*   **Projects & Artifacts:** Sessions manipulate Artifacts on behalf of a Project. A Session connects the temporal reality (when work happened) to the structural reality (what files were changed).
*   **Commitments:** A Session is the execution payload that burns down the required effort of a Commitment.
*   **Knowledge Graph Nodes:** The Session exists as a temporal node in the graph, acting as the structural edge between the user's Timeline and their Projects.

---

## 6. UI Ownership (Layer 6 — Interaction)

Cognitive Sessions are the hidden engine powering the core views of the Chronos UI:

*   **Timeline:** Visualized as blocks of time. Hovering over a session reveals the events inside it.
*   **Cognitive Forensics (Quadrant 03):** When a workflow is abandoned, Forensics analyzes the exact *end boundary* of the last Cognitive Session to explain the interruption.
*   **Workspace Restoration (Quadrant 04):** To restore a workspace, the Execution Layer queries the last known Cognitive Session for a specific project and re-opens the exact artifacts observed inside it.
*   **Mission Control (Quadrant 01 & 02):** Risk forecasting uses the duration and frequency of past Cognitive Sessions to calculate historical velocity and predict future completion chances.
*   **Explainability:** If the user asks, "Why did you suggest this plan?", the system points to the temporal gaps between historical Cognitive Sessions as evidence of drift.
*   **Continuation Planning:** The AI Gateway synthesizes recovery plans by analyzing what was left unfinished at the trailing edge of the most recent Cognitive Session.
