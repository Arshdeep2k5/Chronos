# Commitment Engine Specification

The Commitment Engine is a Layer 3 (Reasoning) subsystem. It monitors the materialized `ChronosState`, active `CognitiveSession` history, and `EntityGraph` details, resolving them into explicit `CommitmentCandidate` structures.

---

## 1. Specification

### Consumes
*   `ChronosState` materialized payloads.
*   `CognitiveSession` history (Session Projection).
*   `EntityGraph` resolved entities and relationships.

### Produces
*   `CommitmentCandidate` event streams published onto the `EventBus` under the `CommitmentCandidateResolved` event type.

---

## 2. Commitment Candidate Schema

```json
{
  "commitment_id": "uuid-9999",
  "title": "Commitment candidate: Refactor/maintain artifact src/lib.rs",
  "confidence": 0.85,
  "evidence_ids": ["event-1", "event-2"],
  "originating_sessions": ["sess-1", "sess-2"],
  "originating_entities": ["ent-1"],
  "created_at": "2026-06-28T12:00:00Z",
  "last_activity_at": "2026-06-28T12:05:00Z"
}
```

---

## 3. Resolution Rules (Factual & AI-Free)

1.  **Project Focus Rule**:
    *   *Logic:* Extracts active projects from the `EntityGraph`. If a `Project` node is referenced in $\ge 2$ sessions, resolves a commitment candidate stating active maintenance.
    *   *Confidence:* $0.8 + 0.05 \times \text{number of active sessions}$ (cap at 1.0).
2.  **Artifact Frequency Rule**:
    *   *Logic:* Identifies `Artifact` entities touched in $\ge 2$ sessions. Marks a commitment to refactor or preserve the artifact.
    *   *Confidence:* $0.7 + 0.05 \times \text{number of references}$.

---

## 4. Replay & Persistence Design
*   Like other projections, commitments are deterministic. Running the same stream of sessions and entities will always produce identical commitment candidates with matches on IDs, titles, and confidences. No fuzzy AI models are used.
