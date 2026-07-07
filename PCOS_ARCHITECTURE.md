# Chronos: Personal Context Operating System (PCOS) Architecture

### Governing Principle
> **Chronos exists to preserve and extend human cognitive continuity.**

Chronos is not a chatbot, a task manager, a note-taking application, or an automation platform. It is the operating layer *beneath* them. It continuously observes the digital environment, builds an evolving model of context, reasons over it, and helps the user resume and complete meaningful work without requiring them to reconstruct their mental state from scratch.

Every design decision must satisfy this single question: *"Does this improve cognitive continuity?"*

---

To achieve this, Chronos requires two orthogonal architectural views, much like a traditional OS separates its structural view (kernel, drivers) from its runtime view (processes, interrupts).

## 1. Structural Architecture: Where Computation Happens

Chronos is organized into seven decoupled layers. This defines where every subsystem lives and ensures strict separation of concerns.

**Layer 0 — Infrastructure**  
*The foundational substrate.*  
- Rust Core Daemon, Embedded Supergraph (SQLite, `sqlite-vec`), Security (SQLCipher), AI Gateway, Plugin Substrate, and the **Cognitive Bus** (see below).

**Layer 1 — Perception**  
*The observation layer.*  
- Adapters for DOM, Files, Active Window, Git (local commits/branches), Clipboard, Audio, Calendar, and Email.

**Layer 2 — Memory**  
*The abstraction and decay layer.*  
- Memory Consolidation Engine, Forgetting Engine (biological decay), Entity Resolution Layer.

**Layer 3 — Reasoning**  
*The "Thinking" layer.*  
- Opportunity Detection Engine (ODE), Deadline Discovery Engine (DDE), Personal Capacity Model (PCM), Schedule Drift Engine (SDE), Pattern Discovery Engine, Habit Discovery Engine.

**Layer 4 — Decision**  
*The orchestration chokepoint.*  
- Decision Orchestrator, Meta-Cognition Engine (confidence propagation), Decision Ledger.

**Layer 5 — Execution**  
*Action and modification.*  
- Context Continuation Engine (CCE), Autonomous Research Continuation (ARC), Action Executor, Workflow Engine.

**Layer 6 — Interaction**  
*User presentation and consent.*  
- Mission Control HUD, Command Center Consent Gate, Timeline View, Explainability Panel, Natural Language Console.

---

## 2. Cognitive Runtime Architecture: The Information Lifecycle

This defines *what* happens when reality changes. Every piece of information flows through a universal 9-stage lifecycle:

`Reality → Observe → Interpret → Remember → Predict → Decide → Act → Reflect → Learn`

### Stage 1: Reality
Chronos never directly "sees" reality. Reality consists of PDFs, browser tabs, Git commits, WhatsApp threads, VSCode sessions, and clipboard contents.

### Stage 2: Observation (Perception Layer)
Observations are intentionally *stupid* and objective. The system records raw facts without inference.
*Example:* `{"type":"FileCreated", "path":"assignment.pdf", "timestamp":"10:00AM"}`

### Stage 3: Interpretation (AI Gateway / Reasoning)
This is where AI transforms raw data into meaning. 
*Example:* `Downloaded: COMP451_Final_Project.pdf` → `Interpreted: Assignment, Deadline, University Course, Confidence: High`.

### Stage 4: Knowledge (Memory Layer)
Interpretations are structured into the graph. To keep the graph clean, knowledge is abstracted across four semantic levels:
1. **Observation:** Raw event (*e.g., Opened VSCode*)
2. **Artifact:** Something concrete (*e.g., main.py*)
3. **Concept:** Something understood (*e.g., Chronos Project*)
4. **Commitment:** Something actionable (*e.g., Finish Graph Engine*)
*(Not every observation becomes a commitment. Not every artifact becomes a concept.)*

### Stage 5: Prediction (Reasoning Layer)
Mathematical and heuristic forecasting (PCM, DPE, SDE). 
*Prediction never decides.* It only answers: *"What will probably happen?"* (e.g., *"Risk is High. Delivery in 2 days."*)

### Stage 6: Decision (Decision Layer)
The missing heart. The **Decision Orchestrator** takes evidence from ODE, DPE, and PCM, and determines what Chronos should actually do. This prevents five independent engines from firing simultaneous notifications.

### Stage 7: Action (Execution Layer)
External modifications: CCE, ARC, staging files, triggering HUD notifications.

### Stage 8: Reflection (Reasoning Layer)
Every action generates feedback. 
*Example:* `Suggested Recovery Plan → User ignored → Missed deadline`. The system asks: *Was my prediction correct? Why or why not?*

### Stage 9: Learning (Reasoning / Infrastructure)
Long-term adaptation. Reflection alters the PCM weights, habit thresholds, prompt selections, and decision policies. Chronos improves itself natively over time.

---

## The Cognitive Bus (Event-Driven Architecture)

Instead of tight coupling where engines call each other sequentially, Chronos operates on a **Cognitive Bus**. 

No subsystem knows who consumes its output. Every subsystem publishes events to the bus, and any interested subscriber can react.

**Anti-Pattern:**
`File Watcher → DDE → Knowledge Graph`

**Chronos Event Bus Pattern:**
`Git Adapter → [Cognitive Bus] → PCM, Knowledge Graph, Timeline View, Analytics, Reflection Engine`

This ensures that Chronos remains infinitely extensible. A new plugin can simply subscribe to the Cognitive Bus to ingest global state without requiring modifications to the core daemon.
