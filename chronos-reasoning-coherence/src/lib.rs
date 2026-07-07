//! # Cognitive Coherence Engine (CCE-2)
//!
//! Unifies Intent Canonicalization, Commitment Domain, and Context Continuity Engine
//! into a single globally consistent cognitive state model, resolving cross-layer
//! contradictions and ensuring Chronos maintains a single coherent truth representation.

use chrono::{DateTime, Utc};
use chronos_core::ChronosEvent;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CognitiveStateNode {
    pub cognitive_state_id: String,
    pub linked_intent_nodes: Vec<String>,
    pub linked_commitment_nodes: Vec<String>,
    pub linked_continuity_nodes: Vec<String>,
    pub unified_semantic_representation: String,
    pub global_confidence_score: f64,
    pub conflict_flags: Vec<String>,
    pub resolution_history: Vec<String>,
    pub source_provenance_graph: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CognitiveStateCreatedPayload {
    pub node: CognitiveStateNode,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CognitiveConflictDetectedPayload {
    pub conflict_type: String,
    pub affected_node_ids: Vec<String>,
    pub details: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CognitiveConflictResolvedPayload {
    pub conflict_type: String,
    pub resolved_node_ids: Vec<String>,
    pub resolution_strategy: String,
    pub emitted_state_node: Option<CognitiveStateNode>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CognitiveStateMergedPayload {
    pub target_id: String,
    pub merged_id: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CognitiveStateSplitPayload {
    pub original_id: String,
    pub new_nodes: Vec<CognitiveStateNode>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CognitiveStateRecalibratedPayload {
    pub node_id: String,
    pub new_confidence: f64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct CognitiveStateGraph {
    pub nodes: HashMap<String, CognitiveStateNode>,
    pub detected_conflicts: Vec<CognitiveConflictDetectedPayload>,
    pub resolved_conflicts: Vec<CognitiveConflictResolvedPayload>,
}

impl CognitiveStateGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn apply_event(&mut self, event: &ChronosEvent) {
        match event.event_type.as_str() {
            "CognitiveStateCreated" => {
                if let Ok(payload) = serde_json::from_value::<CognitiveStateCreatedPayload>(event.payload.clone()) {
                    self.nodes.insert(payload.node.cognitive_state_id.clone(), payload.node);
                }
            }
            "CognitiveConflictDetected" => {
                if let Ok(payload) = serde_json::from_value::<CognitiveConflictDetectedPayload>(event.payload.clone()) {
                    self.detected_conflicts.push(payload);
                }
            }
            "CognitiveConflictResolved" => {
                if let Ok(payload) = serde_json::from_value::<CognitiveConflictResolvedPayload>(event.payload.clone()) {
                    self.resolved_conflicts.push(payload.clone());
                    if let Some(node) = payload.emitted_state_node {
                        self.nodes.insert(node.cognitive_state_id.clone(), node);
                    }
                }
            }
            "CognitiveStateMerged" => {
                if let Ok(payload) = serde_json::from_value::<CognitiveStateMergedPayload>(event.payload.clone()) {
                    if let Some(mut merged) = self.nodes.remove(&payload.merged_id) {
                        if let Some(target) = self.nodes.get_mut(&payload.target_id) {
                            target.linked_intent_nodes.append(&mut merged.linked_intent_nodes);
                            target.linked_commitment_nodes.append(&mut merged.linked_commitment_nodes);
                            target.linked_continuity_nodes.append(&mut merged.linked_continuity_nodes);
                            target.global_confidence_score = (target.global_confidence_score + merged.global_confidence_score).min(1.0);
                            target.resolution_history.push(format!("Merged with {}", payload.merged_id));
                        }
                    }
                }
            }
            "CognitiveStateSplit" => {
                if let Ok(payload) = serde_json::from_value::<CognitiveStateSplitPayload>(event.payload.clone()) {
                    self.nodes.remove(&payload.original_id);
                    for node in payload.new_nodes {
                        self.nodes.insert(node.cognitive_state_id.clone(), node);
                    }
                }
            }
            "CognitiveStateRecalibrated" => {
                if let Ok(payload) = serde_json::from_value::<CognitiveStateRecalibratedPayload>(event.payload.clone()) {
                    if let Some(node) = self.nodes.get_mut(&payload.node_id) {
                        node.global_confidence_score = payload.new_confidence;
                    }
                }
            }
            _ => {}
        }
    }
}

pub struct CoherenceEngine;

impl CoherenceEngine {
    /// Detects inconsistencies and reconciles state models to emit coherence events.
    pub fn reconcile(
        graph: &CognitiveStateGraph,
        intents: &chronos_reasoning_intent::CanonicalState,
        commitments: &chronos_reasoning_commitments::CommitmentState,
        continuity: &chronos_reasoning_continuity::ContinuityGraph,
    ) -> Vec<ChronosEvent> {
        let mut coherence_events = Vec::new();

        // Conflict Type 1: Intent-Commitment Mismatch (Intent exists but no commitment formed)
        for (intent_id, intent) in &intents.intents {
            let mut matched = false;
            for commitment in commitments.commitments.values() {
                // Check semantic similarity / content match
                if commitment.content.to_lowercase() == intent.raw_content.to_lowercase() {
                    matched = true;
                    break;
                }
            }

            if !matched {
                let conflict_id = format!("conflict-intent-commitment-{}", intent_id);
                let is_already_detected = graph.detected_conflicts.iter().any(|c| c.affected_node_ids.contains(&conflict_id));

                if !is_already_detected {
                    // Emit Conflict Detected
                    coherence_events.push(ChronosEvent::new(
                        "CognitiveConflictDetected",
                        "CoherenceEngine",
                        serde_json::to_value(CognitiveConflictDetectedPayload {
                            conflict_type: "IntentCommitmentMismatch".to_string(),
                            affected_node_ids: vec![intent_id.clone(), conflict_id.clone()],
                            details: format!("Intent '{}' exists but has no matching commitment.", intent.raw_content),
                            timestamp: Utc::now(),
                        }).unwrap(),
                    ));

                    // Resolve Conflict: Automatically create CognitiveStateNode mapping them
                    let node = CognitiveStateNode {
                        cognitive_state_id: format!("cog-state-{}", intent_id),
                        linked_intent_nodes: vec![intent_id.clone()],
                        linked_commitment_nodes: Vec::new(),
                        linked_continuity_nodes: Vec::new(),
                        unified_semantic_representation: intent.raw_content.clone(),
                        global_confidence_score: intent.confidence_score * 0.9, // slight downgrade
                        conflict_flags: vec!["IntentCommitmentMismatch".to_string()],
                        resolution_history: vec!["Auto-resolution: Created unlinked cognitive state".to_string()],
                        source_provenance_graph: intent.provenance_event_ids.clone(),
                    };

                    coherence_events.push(ChronosEvent::new(
                        "CognitiveConflictResolved",
                        "CoherenceEngine",
                        serde_json::to_value(CognitiveConflictResolvedPayload {
                            conflict_type: "IntentCommitmentMismatch".to_string(),
                            resolved_node_ids: vec![intent_id.clone()],
                            resolution_strategy: "CreateUnlinkedState".to_string(),
                            emitted_state_node: Some(node),
                            timestamp: Utc::now(),
                        }).unwrap(),
                    ));
                }
            }
        }

        coherence_events
    }

    /// Rebuilds the global CognitiveStateGraph from event history.
    pub fn rebuild_cognitive_state(events: &[ChronosEvent]) -> CognitiveStateGraph {
        let mut graph = CognitiveStateGraph::new();
        for event in events {
            graph.apply_event(event);
        }
        graph
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_conflict_detection_and_resolution() {
        let mut intents = chronos_reasoning_intent::CanonicalState::default();
        let commitments = chronos_reasoning_commitments::CommitmentState::default();
        let continuity = chronos_reasoning_continuity::ContinuityGraph::default();

        // Populate intent
        let intent_id = "test-intent-id".to_string();
        intents.intents.insert(
            intent_id.clone(),
            chronos_reasoning_intent::IntentSignal {
                intent_id: intent_id.clone(),
                source: "manual".to_string(),
                raw_content: "Refactor API".to_string(),
                signal_type: chronos_reasoning_intent::SignalType::Explicit,
                timestamp: Utc::now(),
                confidence_score: 0.95,
                context_metadata: HashMap::new(),
                provenance_event_ids: Vec::new(),
            },
        );

        let mut cog_graph = CognitiveStateGraph::new();

        // Perform reconcile
        let events = CoherenceEngine::reconcile(&cog_graph, &intents, &commitments, &continuity);
        
        // Assert conflict is detected and resolved
        assert!(events.iter().any(|e| e.event_type == "CognitiveConflictDetected"));
        assert!(events.iter().any(|e| e.event_type == "CognitiveConflictResolved"));

        // Apply events to graph
        for ev in &events {
            cog_graph.apply_event(ev);
        }

        assert!(cog_graph.nodes.contains_key(&format!("cog-state-{}", intent_id)));

        // Replay/Reconstruct sanity check
        let replayed_graph = CoherenceEngine::rebuild_cognitive_state(&events);
        assert_eq!(cog_graph.nodes.len(), replayed_graph.nodes.len());
    }
}
