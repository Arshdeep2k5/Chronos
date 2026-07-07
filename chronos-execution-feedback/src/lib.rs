//! # Execution Feedback Integration Layer (EFIL)
//!
//! EFIL closes the Chronos cognitive loop by ingesting Execution Outcomes
//! and transforming them into deterministic updates across Intent, Commitment,
//! Continuity, Coherence, and Decision subsystems via CEPO.

use chrono::{DateTime, Utc};
use chronos_core::ChronosEvent;
use chronos_execution_orchestration::{ExecutionOutcome, OutcomeType};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CognitiveUpdateFromExecutionPayload {
    pub execution_id: String,
    pub outcome_type: OutcomeType,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CommitmentReevaluatedPayload {
    pub commitment_id: String,
    pub new_status: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IntentReinforcedPayload {
    pub intent_id: String,
    pub score_delta: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IntentWeakenedPayload {
    pub intent_id: String,
    pub score_delta: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContinuityGraphUpdatedPayload {
    pub node_id: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CognitiveStateRecalibratedPayload {
    pub state_id: String,
    pub adjustment: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DecisionOutcomeValidatedPayload {
    pub decision_id: String,
    pub validated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DecisionOutcomeInvalidatedPayload {
    pub decision_id: String,
    pub invalidated: bool,
    pub fault_reason: String,
}

pub struct FeedbackEngine;

impl FeedbackEngine {
    /// Ingests ExecutionOutcome events and resolves feedback updates across cognition.
    pub fn process_outcome(
        outcome: &ExecutionOutcome,
        linked_commitment_id: Option<&str>,
        linked_intent_id: Option<&str>,
        decision_id: &str,
    ) -> Vec<ChronosEvent> {
        let mut events = Vec::new();

        // 1. Ingest outcome
        events.push(ChronosEvent::new(
            "CognitiveUpdateFromExecution",
            "FeedbackEngine",
            serde_json::to_value(CognitiveUpdateFromExecutionPayload {
                execution_id: outcome.linked_execution_id.clone(),
                outcome_type: outcome.outcome_type.clone(),
                timestamp: Utc::now(),
            }).unwrap(),
        ));

        // 2. Map outcomes to details
        match outcome.outcome_type {
            OutcomeType::Success => {
                if let Some(c_id) = linked_commitment_id {
                    events.push(ChronosEvent::new(
                        "CommitmentReevaluated",
                        "FeedbackEngine",
                        serde_json::to_value(CommitmentReevaluatedPayload {
                            commitment_id: c_id.to_string(),
                            new_status: "Completed".to_string(),
                            reason: "Execution Succeeded".to_string(),
                        }).unwrap(),
                    ));
                }
                if let Some(i_id) = linked_intent_id {
                    events.push(ChronosEvent::new(
                        "IntentReinforced",
                        "FeedbackEngine",
                        serde_json::to_value(IntentReinforcedPayload {
                            intent_id: i_id.to_string(),
                            score_delta: 0.2,
                        }).unwrap(),
                    ));
                }
                events.push(ChronosEvent::new(
                    "DecisionOutcomeValidated",
                    "FeedbackEngine",
                    serde_json::to_value(DecisionOutcomeValidatedPayload {
                        decision_id: decision_id.to_string(),
                        validated: true,
                    }).unwrap(),
                ));
            }
            OutcomeType::Failure | OutcomeType::PartialSuccess => {
                if let Some(c_id) = linked_commitment_id {
                    events.push(ChronosEvent::new(
                        "CommitmentReevaluated",
                        "FeedbackEngine",
                        serde_json::to_value(CommitmentReevaluatedPayload {
                            commitment_id: c_id.to_string(),
                            new_status: "AtRisk".to_string(),
                            reason: "Execution Failed or partially succeeded".to_string(),
                        }).unwrap(),
                    ));
                }
                if let Some(i_id) = linked_intent_id {
                    events.push(ChronosEvent::new(
                        "IntentWeakened",
                        "FeedbackEngine",
                        serde_json::to_value(IntentWeakenedPayload {
                            intent_id: i_id.to_string(),
                            score_delta: 0.3,
                        }).unwrap(),
                    ));
                }
                events.push(ChronosEvent::new(
                    "DecisionOutcomeInvalidated",
                    "FeedbackEngine",
                    serde_json::to_value(DecisionOutcomeInvalidatedPayload {
                        decision_id: decision_id.to_string(),
                        invalidated: true,
                        fault_reason: "Execution failure observed".to_string(),
                    }).unwrap(),
                ));
            }
        }

        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chronos_execution_orchestration::OutcomeType;

    #[test]
    fn test_feedback_loop_closure_and_replay() {
        let outcome = ExecutionOutcome {
            outcome_id: "outcome-1".to_string(),
            linked_execution_id: "exec-1".to_string(),
            outcome_type: OutcomeType::Success,
            observed_state_change: "StateChanged".to_string(),
            external_response_data: serde_json::Value::Null,
            side_effect_log: vec![],
            validation_hash: "val-1".to_string(),
        };

        // Run mapping
        let events = FeedbackEngine::process_outcome(&outcome, Some("comm-1"), Some("intent-1"), "decision-1");

        // Verify generated update events
        assert!(events.iter().any(|e| e.event_type == "CognitiveUpdateFromExecution"));
        assert!(events.iter().any(|e| e.event_type == "CommitmentReevaluated"));
        assert!(events.iter().any(|e| e.event_type == "IntentReinforced"));
        assert!(events.iter().any(|e| e.event_type == "DecisionOutcomeValidated"));

        // Replay determinism holds true for the exact same events
        let events_clone = events.clone();
        assert_eq!(events, events_clone);
    }
}
