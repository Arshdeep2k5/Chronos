# FRONTEND_ARCHITECTURE.md
*Authoritative Frontend Single Page Application (SPA) Specification*

---

## 1. Application Layout & Component Hierarchy

The active frontend resides in `chronos-pilot/src/` and is structured as a single dashboard workspace.

```
App.tsx (Main Entrypoint)
 ├── CSL State Manager (semanticInterpreter, stabilityModel)
 ├── MissionControlHUD Layout (Grid)
 │    ├── ARCPanel (Layer 5: execution tasks log)
 │    ├── RiskForecaster (Layer 3: svg risk curves)
 │    ├── CommitmentList (Layer 3: obligations, plan controls)
 │    ├── WorkspaceRestorer (Layer 5: workspace snapshot restore cards)
 │    └── InteractiveSandbox (Perception test console)
 │
 └── DecisionVisibilityPanel (CSL Explainer)
      └── CognitiveGraphView (Mermaid/SVG Physics-based Topology Graph)
```

---

## 2. Event Sourcing Reconstruction (State Management)

Unlike typical web apps that query relational SQL tables directly, `chronos-pilot` implements client-side event sourcing:
1.  **Ingestion Channel**: The application mounts an EventSource connection to `/api/events/stream/live` (*Evidence: `App.tsx#L344`*).
2.  **State Projection Reducer**: As events stream in, the frontend runs `rebuildState` to construct local arrays of commitments, actions, and projects dynamically in memory (*Evidence: `App.tsx#L82`*).
3.  **Local Database Viewer Parsing**: The `DatabaseViewer.tsx` component parses client-side event arrays inside React instead of querying SQLite tables directly (*Evidence: `DatabaseViewer.tsx#L34`*).

---

## 3. Cognitive Semantic Layer (CSL) & Visualization Pipeline

The local Cognitive Semantic Layer (CSL) and SVG Force-Directed Graph operate in tandem:

```
[Incoming SSE Event Stream]
            │
            ▼
┌───────────────────────┐
│  rebuildState Reducer │
└───────────┬───────────┘
            │ Updates
            ▼
┌───────────────────────┐
│ /cognitive Calculator │
│  (intent, stability,  │
│   execution pressure) │
└───────────┬───────────┘
            │ Calculations
            ▼
┌─────────────────────────┐
│  CognitiveGraphView     │
│  (SVG Force-Directed    │
│   Physics Simulation)   │
└─────────────────────────┘
```

*   **CSL Computation**: Computes stability indices, drift velocities, reasoning complexity, and execution pressure values locally inside the client SPA.
*   **Force-Directed Graph**: Maps derived CSL metrics (Intents, Risks, Commitments) to nodes, and relationships (Causality, Temporal adjacencies) to edges on an SVG viewport with drag-and-drop support.

---

## 4. UI Migration Status

*   **`ui/` (Legacy React UI)**: The original Tauri React UI, currently kept as static code. Bypassed by the active Vite server configuration.
*   **`chronos-pilot/` (Active React UI)**: Fully connected to the new PCOS API bridge server on port 7899.

---
