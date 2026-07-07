//! # Chronos Context Continuity Engine (CCE)
//!
//! Provides cross-session temporal continuity for intents and commitments,
//! ensuring Chronos maintains stable long-horizon understanding of user obligations,
//! unresolved work, and evolving intent across time.

use chrono::{DateTime, Utc};
use chronos_core::ChronosEvent;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PersistentIntentStatus {
    Active,
    Dormant,
    Resurfaced,
    Obsolete,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PersistentIntentNode {
    pub persistent_intent_id: String,
    pub linked_intent_ids: Vec<String>,
    pub canonical_content: String,
    pub first_seen_timestamp: DateTime<Utc>,
    pub last_seen_timestamp: DateTime<Utc>,
    pub reinforcement_score: f64,
    pub decay_score: f64,
    pub source_distribution: HashMap<String, u32>,
    pub status: PersistentIntentStatus,
    pub provenance_event_chain: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PersistentCommitmentNode {
    pub persistent_commitment_id: String,
    pub linked_commitment_ids: Vec<String>,
    pub canonical_commitment_reference: String,
    pub stability_score: f64,
    pub drift_score: f64,
    pub deadline_evolution_history: Vec<(DateTime<Utc>, Option<DateTime<Utc>>)>,
    pub state_trajectory_across_sessions: Vec<(DateTime<Utc>, String)>,
    pub provenance: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PersistentIntentCreatedPayload {
    pub node: PersistentIntentNode,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PersistentIntentReinforcedPayload {
    pub persistent_intent_id: String,
    pub added_strength: f64,
    pub timestamp: DateTime<Utc>,
    pub provenance_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PersistentIntentDecayedPayload {
    pub persistent_intent_id: String,
    pub decay_amount: f64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PersistentIntentMergedPayload {
    pub target_persistent_intent_id: String,
    pub merged_persistent_intent_id: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PersistentCommitmentEvolvedPayload {
    pub persistent_commitment_id: String,
    pub updated_node: PersistentCommitmentNode,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PersistentCommitmentDriftDetectedPayload {
    pub persistent_commitment_id: String,
    pub drift_amount: f64,
    pub details: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SessionLinkedToPersistentGraphPayload {
    pub session_id: String,
    pub persistent_node_ids: Vec<String>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ContinuityGraph {
    pub intents: HashMap<String, PersistentIntentNode>,
    pub commitments: HashMap<String, PersistentCommitmentNode>,
    pub session_links: HashMap<String, Vec<String>>,
}

impl ContinuityGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn apply_event(&mut self, event: &ChronosEvent) {
        match event.event_type.as_str() {
            "PersistentIntentCreated" => {
                if let Ok(payload) = serde_json::from_value::<PersistentIntentCreatedPayload>(event.payload.clone()) {
                    self.intents.insert(payload.node.persistent_intent_id.clone(), payload.node);
                }
            }
            "PersistentIntentReinforced" => {
                if let Ok(payload) = serde_json::from_value::<PersistentIntentReinforcedPayload>(event.payload.clone()) {
                    if let Some(intent) = self.intents.get_mut(&payload.persistent_intent_id) {
                        intent.reinforcement_score += payload.added_strength;
                        intent.last_seen_timestamp = payload.timestamp;
                        intent.provenance_event_chain.push(payload.provenance_id);
                        if intent.status == PersistentIntentStatus::Dormant {
                            intent.status = PersistentIntentStatus::Resurfaced;
                        }
                    }
                }
            }
            "PersistentIntentDecayed" => {
                if let Ok(payload) = serde_json::from_value::<PersistentIntentDecayedPayload>(event.payload.clone()) {
                    if let Some(intent) = self.intents.get_mut(&payload.persistent_intent_id) {
                        intent.decay_score += payload.decay_amount;
                        if intent.reinforcement_score - intent.decay_score <= 0.2 {
                            intent.status = PersistentIntentStatus::Dormant;
                        }
                    }
                }
            }
            "PersistentIntentMerged" => {
                if let Ok(payload) = serde_json::from_value::<PersistentIntentMergedPayload>(event.payload.clone()) {
                    if let Some(mut merged) = self.intents.remove(&payload.merged_persistent_intent_id) {
                        if let Some(target) = self.intents.get_mut(&payload.target_persistent_intent_id) {
                            target.linked_intent_ids.append(&mut merged.linked_intent_ids);
                            target.reinforcement_score = (target.reinforcement_score + merged.reinforcement_score).min(2.0);
                            target.provenance_event_chain.append(&mut merged.provenance_event_chain);
                            target.last_seen_timestamp = payload.timestamp;
                        }
                    }
                }
            }
            "PersistentCommitmentEvolved" => {
                if let Ok(payload) = serde_json::from_value::<PersistentCommitmentEvolvedPayload>(event.payload.clone()) {
                    self.commitments.insert(payload.persistent_commitment_id, payload.updated_node);
                }
            }
            "PersistentCommitmentDriftDetected" => {
                if let Ok(payload) = serde_json::from_value::<PersistentCommitmentDriftDetectedPayload>(event.payload.clone()) {
                    if let Some(comm) = self.commitments.get_mut(&payload.persistent_commitment_id) {
                        comm.drift_score = payload.drift_amount;
                    }
                }
            }
            "SessionLinkedToPersistentGraph" => {
                if let Ok(payload) = serde_json::from_value::<SessionLinkedToPersistentGraphPayload>(event.payload.clone()) {
                    self.session_links.insert(payload.session_id, payload.persistent_node_ids);
                }
            }
            _ => {}
        }
    }
}

pub struct ContextContinuityEngine;

impl ContextContinuityEngine {
    /// Evaluates canonicalized events to yield cross-session continuity events.
    pub fn process_canonical_events(
        graph: &ContinuityGraph,
        events: &[ChronosEvent],
        session_id: &str,
    ) -> Vec<ChronosEvent> {
        let mut continuity_events = Vec::new();
        let mut linked_ids = Vec::new();

        for event in events {
            match event.event_type.as_str() {
                "CommitmentCanonicalized" => {
                    // Create or evolve persistent commitments
                    if let Ok(payload) = serde_json::from_value::<CommitmentCanonicalizedPayload>(event.payload.clone()) {
                        let c = payload.canonical_commitment.commitment;
                        let p_id = format!("persistent-commitment-{}", c.commitment_id);
                        
                        let mut updated_node = if let Some(existing) = graph.commitments.get(&p_id) {
                            existing.clone()
                        } else {
                            PersistentCommitmentNode {
                                persistent_commitment_id: p_id.clone(),
                                linked_commitment_ids: Vec::new(),
                                canonical_commitment_reference: c.content.clone(),
                                stability_score: 0.5,
                                drift_score: 0.0,
                                deadline_evolution_history: Vec::new(),
                                state_trajectory_across_sessions: Vec::new(),
                                provenance: Vec::new(),
                            }
                        };

                        updated_node.linked_commitment_ids.push(c.commitment_id.clone());
                        updated_node.deadline_evolution_history.push((event.timestamp, c.inferred_due_at));
                        updated_node.state_trajectory_across_sessions.push((event.timestamp, format!("{:?}", c.status)));
                        updated_node.provenance.push(event.id.clone());

                        continuity_events.push(ChronosEvent::new(
                            "PersistentCommitmentEvolved",
                            "ContextContinuityEngine",
                            serde_json::to_value(PersistentCommitmentEvolvedPayload {
                                persistent_commitment_id: p_id.clone(),
                                updated_node,
                            }).unwrap(),
                        ));
                        linked_ids.push(p_id);
                    }
                }
                "IntentDetected" => {
                    if let Ok(payload) = serde_json::from_value::<IntentDetectedPayload>(event.payload.clone()) {
                        let intent = payload.intent;
                        let p_intent_id = format!("persistent-intent-{}", intent.intent_id);

                        // Check semantic similarity with existing persistent intents for cross-session merge
                        let mut merge_target = None;
                        for existing in graph.intents.values() {
                            if existing.canonical_content.to_lowercase() == intent.raw_content.to_lowercase() {
                                merge_target = Some(existing.persistent_intent_id.clone());
                                break;
                            }
                        }

                        if let Some(target_id) = merge_target {
                            continuity_events.push(ChronosEvent::new(
                                "PersistentIntentReinforced",
                                "ContextContinuityEngine",
                                serde_json::to_value(PersistentIntentReinforcedPayload {
                                    persistent_intent_id: target_id.clone(),
                                    added_strength: 0.2,
                                    timestamp: event.timestamp,
                                    provenance_id: event.id.clone(),
                                }).unwrap(),
                            ));
                            linked_ids.push(target_id);
                        } else {
                            let mut source_dist = HashMap::new();
                            source_dist.insert(intent.source.clone(), 1);

                            let node = PersistentIntentNode {
                                persistent_intent_id: p_intent_id.clone(),
                                linked_intent_ids: vec![intent.intent_id.clone()],
                                canonical_content: intent.raw_content.clone(),
                                first_seen_timestamp: event.timestamp,
                                last_seen_timestamp: event.timestamp,
                                reinforcement_score: intent.confidence_score,
                                decay_score: 0.0,
                                source_distribution: source_dist,
                                status: PersistentIntentStatus::Active,
                                provenance_event_chain: vec![event.id.clone()],
                            };

                            continuity_events.push(ChronosEvent::new(
                                "PersistentIntentCreated",
                                "ContextContinuityEngine",
                                serde_json::to_value(PersistentIntentCreatedPayload { node }).unwrap(),
                            ));
                            linked_ids.push(p_intent_id);
                        }
                    }
                }
                _ => {}
            }
        }

        if !linked_ids.is_empty() {
            continuity_events.push(ChronosEvent::new(
                "SessionLinkedToPersistentGraph",
                "ContextContinuityEngine",
                serde_json::to_value(SessionLinkedToPersistentGraphPayload {
                    session_id: session_id.to_string(),
                    persistent_node_ids: linked_ids,
                    timestamp: Utc::now(),
                }).unwrap(),
            ));
        }

        continuity_events
    }

    /// Rebuilds the continuity graph from event history.
    pub fn rebuild_continuity_graph(events: &[ChronosEvent]) -> ContinuityGraph {
        let mut graph = ContinuityGraph::new();
        for event in events {
            graph.apply_event(event);
        }
        graph
    }
}

// Temporary compatibility structure
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CommitmentCanonicalizedPayload {
    pub canonical_commitment: chronos_reasoning_intent::CanonicalCommitment,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct IntentDetectedPayload {
    pub intent: chronos_reasoning_intent::IntentSignal,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_cross_session_continuity_and_replay() {
        let mut graph = ContinuityGraph::new();
        let timestamp = Utc::now();

        // 1. Session 1: Intent Detected
        let event1 = ChronosEvent::new(
            "IntentDetected",
            "Test",
            json!({
                "intent": {
                    "intent_id": "intent-1",
                    "source": "manual",
                    "raw_content": "Build compiler",
                    "signal_type": "Explicit",
                    "timestamp": timestamp,
                    "confidence_score": 0.9,
                    "context_metadata": {},
                    "provenance_event_ids": []
                }
            }),
        );

        let out1 = ContextContinuityEngine::process_canonical_events(&graph, &[event1], "session-1");
        for ev in &out1 {
            graph.apply_event(ev);
        }

        // PersistentIntent should be created
        assert!(graph.intents.contains_key("persistent-intent-intent-1"));

        // 2. Session 2: Same semantic intent resurfaces
        let event2 = ChronosEvent::new(
            "IntentDetected",
            "Test",
            json!({
                "intent": {
                    "intent_id": "intent-2",
                    "source": "manual",
                    "raw_content": "Build compiler",
                    "signal_type": "Explicit",
                    "timestamp": timestamp + chrono::Duration::hours(1),
                    "confidence_score": 0.9,
                    "context_metadata": {},
                    "provenance_event_ids": []
                }
            }),
        );

        let out2 = ContextContinuityEngine::process_canonical_events(&graph, &[event2], "session-2");
        for ev in &out2 {
            graph.apply_event(ev);
        }

        // Should reinforce instead of creating new node
        let p_node = graph.intents.get("persistent-intent-intent-1").unwrap();
        assert!(p_node.reinforcement_score > 0.9);

        // Verify replay determinism
        let mut all_events = Vec::new();
        all_events.extend(out1);
        all_events.extend(out2);

        let replayed_graph = ContextContinuityEngine::rebuild_continuity_graph(&all_events);
        assert_eq!(graph, replayed_graph);
    }
}
