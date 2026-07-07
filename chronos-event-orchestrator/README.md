# Chronos Event Processing Orchestrator (CEPO)

CEPO unifies the runtime event flow between Reality Ingestion, Intent Canonicalization, and the Commitment Domain systems into a single deterministic, replay-safe execution pipeline.

## Pipeline Architecture

```mermaid
flowchart TD
    RawEvent[Raw Perception Event] -->|normalize| IntentSignal[IntentSignal]
    IntentSignal -->|process| Merged{Merged?}
    Merged -->|Yes| IntentMerged[IntentMerged Event]
    Merged -->|No| Deduplicated{Duplicated?}
    Deduplicated -->|Yes| CommitmentDeDuplicated[CommitmentDeDuplicated Event]
    Deduplicated -->|No| CommitmentCanonicalized[CommitmentCanonicalized Event]
    
    CommitmentCanonicalized -->|Map| CommitmentDiscovered[CommitmentDiscovered Event]
    CommitmentDeDuplicated -->|Map| CommitmentUpdated[CommitmentUpdated Event]
    
    CommitmentDiscovered --> SQLite[SQLite EventStore & Cognitive Bus]
    CommitmentUpdated --> SQLite
```

## Guarantees & Invariants
- **Deterministic Pipeline Execution**: The orchestrator is the single entry point for event flows. Input sequences produce identical derived outputs.
- **Replay Determinism**: The `rebuild_from_history` function uses the state reducer logic (`apply_event`) directly without regenerating derived events to guarantee idempotency and identical reconstructed state on start-up warmups.
