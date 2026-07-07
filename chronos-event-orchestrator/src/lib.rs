//! # Chronos Event Processing Orchestrator (CEPO)
//!
//! Unifies the runtime event flow between Reality Capture, Intent Canonicalization,
//! and Commitment Domain systems into a single deterministic, replay-safe execution pipeline.

use chronos_core::ChronosEvent;
use chronos_reasoning_intent::{
    CanonicalCommitmentBuilder, CanonicalState, CommitmentCanonicalizedPayload,
    CommitmentDeDuplicatedPayload,
};
use chronos_reasoning_commitments::{CommitmentDiscoveredPayload, CommitmentState};
use serde_json::json;

pub struct EventOrchestrator {
    pub intent_state: CanonicalState,
    pub commitment_state: CommitmentState,
}

impl EventOrchestrator {
    pub fn new() -> Self {
        Self {
            intent_state: CanonicalState::new(),
            commitment_state: CommitmentState::new(),
        }
    }

    /// Unifies raw events and lifecycle events sequentially.
    /// Returns the sequence of all output/derived events.
    pub fn process_event(&mut self, event: ChronosEvent) -> Result<Vec<ChronosEvent>, anyhow::Error> {
        let mut output_events = Vec::new();

        let is_raw = matches!(
            event.event_type.as_str(),
            "BrowserUrlChanged"
                | "BrowserTabActivated"
                | "EditorFileOpened"
                | "GitCommitCreated"
                | "ManualContextIngested"
        );

        if is_raw {
            // 1. Raw event is added first
            output_events.push(event.clone());

            // 2. Normalize raw event to IntentSignal
            if let Some(intent) = CanonicalCommitmentBuilder::normalize_event(&event) {
                // 3. Process new intent against current intent_state
                let intent_events = CanonicalCommitmentBuilder::process_new_intent(&self.intent_state, intent);
                for ie in intent_events {
                    // Apply to internal intent projection
                    self.intent_state.apply_event(&ie);
                    output_events.push(ie.clone());

                    // Map to Commitment Domain layer events
                    match ie.event_type.as_str() {
                        "CommitmentCanonicalized" => {
                            if let Ok(payload) = serde_json::from_value::<CommitmentCanonicalizedPayload>(ie.payload.clone()) {
                                let c = payload.canonical_commitment.commitment;
                                let disco_payload = CommitmentDiscoveredPayload {
                                    commitment_id: c.commitment_id.clone(),
                                    source: c.source.clone(),
                                    content: c.content.clone(),
                                    created_at: c.created_at,
                                    inferred_due_at: c.inferred_due_at,
                                    confidence: c.confidence,
                                };
                                let disco_ev = ChronosEvent::new(
                                    "CommitmentDiscovered",
                                    "EventOrchestrator",
                                    serde_json::to_value(disco_payload).unwrap(),
                                );
                                // Apply to commitment state
                                self.commitment_state.apply_event(&disco_ev);
                                output_events.push(disco_ev);
                            }
                        }
                        "CommitmentDeDuplicated" => {
                            if let Ok(payload) = serde_json::from_value::<CommitmentDeDuplicatedPayload>(ie.payload.clone()) {
                                let update_ev = ChronosEvent::new(
                                    "CommitmentUpdated",
                                    "EventOrchestrator",
                                    json!({
                                        "commitment_id": payload.deduplicated_with_id,
                                        "confidence": 1.0,
                                    }),
                                );
                                self.commitment_state.apply_event(&update_ev);
                                output_events.push(update_ev);
                            }
                        }
                        _ => {}
                    }
                }
            }
        } else {
            // Apply lifecycle/downstream events directly to state for replay consistency
            self.intent_state.apply_event(&event);
            self.commitment_state.apply_event(&event);
            output_events.push(event);
        }

        Ok(output_events)
    }

    /// Replays a history of events deterministically.
    pub fn rebuild_from_history(&mut self, history: Vec<ChronosEvent>) {
        for event in history {
            self.intent_state.apply_event(&event);
            self.commitment_state.apply_event(&event);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_orchestrator_pipeline_end_to_end() {
        let mut cepo = EventOrchestrator::new();
        let raw_event = ChronosEvent::new(
            "ManualContextIngested",
            "Manual",
            json!({ "display_name": "Buy milk today" }),
        );

        let out = cepo.process_event(raw_event).unwrap();
        
        // Assert: IntentDetected, CommitmentCanonicalized, and CommitmentDiscovered are produced
        assert!(out.iter().any(|e| e.event_type == "IntentDetected"));
        assert!(out.iter().any(|e| e.event_type == "CommitmentCanonicalized"));
        assert!(out.iter().any(|e| e.event_type == "CommitmentDiscovered"));

        // State validation
        assert_eq!(cepo.commitment_state.commitments.len(), 1);
    }

    #[test]
    fn test_replay_determinism() {
        let mut cepo1 = EventOrchestrator::new();
        let mut cepo2 = EventOrchestrator::new();

        let raw_events = vec![
            ChronosEvent::new(
                "ManualContextIngested",
                "Manual",
                json!({ "display_name": "Write code" }),
            ),
            ChronosEvent::new(
                "ManualContextIngested",
                "Manual",
                json!({ "display_name": "Write code" }),
            ),
        ];

        let mut out1 = Vec::new();
        for ev in raw_events.clone() {
            let mut res = cepo1.process_event(ev).unwrap();
            out1.append(&mut res);
        }

        // Replay output stream through second CEPO
        cepo2.rebuild_from_history(out1.clone());

        assert_eq!(cepo1.commitment_state.commitments, cepo2.commitment_state.commitments);
    }
}
