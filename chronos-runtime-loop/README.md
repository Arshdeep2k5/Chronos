# Continuous Runtime Loop Engine (CRLE)

CRLE transitions Chronos from run-on-demand pipeline runs to a continuous, deterministic cognitive runtime loop that perpetually processes incoming event streams.

## Runtime Tick Phases

Each tick executes the following phases in strict order:
1. **Ingestion**: Polls and cursor-consumes batches of events from the SQLiteEventStore (CEPO stream).
2. **Cognitive Update**: Rebuilds intent and commitment state projections and runs coherence checks to emit reconcile events.
3. **Decision Selection**: Recomputes prioritized Decision Candidates and selects primary action nodes.
4. **Execution Scheduling**: Translates selected decisions into deterministic Execution Plans.
5. **Execution Dispatch**: Runs execution plans through adapters or simulation wrappers in Replay mode.
6. **Feedback Processing**: Ingests Execution Outcomes to reinforce memory weights.

## Replay Guarantees
Replay mode disables real-world side effects, simulating EOL execution steps. Since all state changes are derived from CEPO event streams, replaying identical inputs guarantees identical cognitive graphs.
