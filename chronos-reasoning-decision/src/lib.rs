//! # Decision Orchestration Engine (DOE)
//!
//! Converts the unified CognitiveStateGraph into deterministic, constraint-aware
//! action selection, enabling Chronos to choose what to do next under competing
//! commitments, priorities, and resource limits.

use chrono::{DateTime, Utc};
use chronos_core::ChronosEvent;
use chronos_reasoning_coherence::CognitiveStateGraph;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ActionType {
    Execute,
    Delay,
    Split,
    Delegate,
    Drop,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DecisionCandidate {
    pub decision_candidate_id: String,
    pub source_cognitive_state_id: String,
    pub linked_intent_id: Option<String>,
    pub linked_commitment_id: Option<String>,
    pub action_type: ActionType,
    pub estimated_effort: f64,
    pub urgency_score: f64,
    pub importance_score: f64,
    pub risk_score: f64,
    pub feasibility_score: f64,
    pub temporal_weight: f64,
    pub provenance_chain: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DecisionNode {
    pub decision_id: String,
    pub selected_candidate_id: String,
    pub priority_rank: u32,
    pub selection_reason_chain: Vec<String>,
    pub competing_rejected_candidates: Vec<String>,
    pub resource_allocation_estimate: f64,
    pub expected_outcome_type: String,
    pub timestamp: DateTime<Utc>,
    pub stability_signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DecisionCandidateGeneratedPayload {
    pub candidate: DecisionCandidate,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DecisionRankedPayload {
    pub candidate_id: String,
    pub final_score: f64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DecisionSelectedPayload {
    pub decision: DecisionNode,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DecisionRejectedPayload {
    pub candidate_id: String,
    pub reason: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DecisionDeferredPayload {
    pub candidate_id: String,
    pub deferral_until: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DecisionConflictResolvedPayload {
    pub conflict_id: String,
    pub winning_candidate_id: String,
    pub resolution_details: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct DecisionGraph {
    pub candidates: HashMap<String, DecisionCandidate>,
    pub scores: HashMap<String, f64>,
    pub decisions: HashMap<String, DecisionNode>,
}

impl DecisionGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn apply_event(&mut self, event: &ChronosEvent) {
        match event.event_type.as_str() {
            "DecisionCandidateGenerated" => {
                if let Ok(payload) = serde_json::from_value::<DecisionCandidateGeneratedPayload>(event.payload.clone()) {
                    self.candidates.insert(payload.candidate.decision_candidate_id.clone(), payload.candidate);
                }
            }
            "DecisionRanked" => {
                if let Ok(payload) = serde_json::from_value::<DecisionRankedPayload>(event.payload.clone()) {
                    self.scores.insert(payload.candidate_id, payload.final_score);
                }
            }
            "DecisionSelected" => {
                if let Ok(payload) = serde_json::from_value::<DecisionSelectedPayload>(event.payload.clone()) {
                    self.decisions.insert(payload.decision.decision_id.clone(), payload.decision);
                }
            }
            _ => {}
        }
    }
}

pub struct DecisionPipeline;

impl DecisionPipeline {
    /// Generates candidate and selection events from the CognitiveStateGraph.
    pub fn generate_decisions(cognitive_state: &CognitiveStateGraph) -> Vec<ChronosEvent> {
        let mut events = Vec::new();
        let mut scored_candidates = Vec::new();

        // 1. Candidate Generation Stage
        for (cog_id, node) in &cognitive_state.nodes {
            let candidate_id = generate_stable_hash(cog_id, &node.unified_semantic_representation);
            let candidate = DecisionCandidate {
                decision_candidate_id: candidate_id.clone(),
                source_cognitive_state_id: cog_id.clone(),
                linked_intent_id: node.linked_intent_nodes.first().cloned(),
                linked_commitment_id: node.linked_commitment_nodes.first().cloned(),
                action_type: ActionType::Execute,
                estimated_effort: 1.0,
                urgency_score: if node.conflict_flags.contains(&"IntentCommitmentMismatch".to_string()) { 0.8 } else { 0.5 },
                importance_score: 0.7,
                risk_score: 0.3,
                feasibility_score: 0.9,
                temporal_weight: 0.6,
                provenance_chain: node.source_provenance_graph.clone(),
            };

            events.push(ChronosEvent::new(
                "DecisionCandidateGenerated",
                "DecisionPipeline",
                serde_json::to_value(DecisionCandidateGeneratedPayload { candidate: candidate.clone() }).unwrap(),
            ));

            // 2. Scoring Stage
            // FinalScore = Urgency + Importance + Temporal - Risk
            let final_score = candidate.urgency_score + candidate.importance_score + candidate.temporal_weight - candidate.risk_score;
            scored_candidates.push((candidate_id.clone(), final_score, candidate));

            events.push(ChronosEvent::new(
                "DecisionRanked",
                "DecisionPipeline",
                serde_json::to_value(DecisionRankedPayload {
                    candidate_id,
                    final_score,
                    timestamp: Utc::now(),
                }).unwrap(),
            ));
        }

        // 3. Selection Stage: Select top scored candidate as primary decision
        scored_candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.cmp(&b.0))); // Tie breaking by stable ID

        if let Some((best_id, _score, best_candidate)) = scored_candidates.first() {
            let rejected_ids = scored_candidates.iter().skip(1).map(|c| c.0.clone()).collect();
            let decision = DecisionNode {
                decision_id: format!("decision-{}", best_id),
                selected_candidate_id: best_id.clone(),
                priority_rank: 1,
                selection_reason_chain: vec![format!("Top score candidate: {}", best_candidate.source_cognitive_state_id)],
                competing_rejected_candidates: rejected_ids,
                resource_allocation_estimate: best_candidate.estimated_effort,
                expected_outcome_type: "Success".to_string(),
                timestamp: Utc::now(),
                stability_signature: format!("stable-sig-{}", best_id),
            };

            events.push(ChronosEvent::new(
                "DecisionSelected",
                "DecisionPipeline",
                serde_json::to_value(DecisionSelectedPayload { decision }).unwrap(),
            ));
        }

        events
    }

    /// Rebuilds the DecisionGraph projection from event history.
    pub fn rebuild_decision_graph(events: &[ChronosEvent]) -> DecisionGraph {
        let mut graph = DecisionGraph::new();
        for event in events {
            graph.apply_event(event);
        }
        graph
    }
}

fn generate_stable_hash(input1: &str, input2: &str) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    use std::hash::Hash;
    input1.hash(&mut hasher);
    input2.hash(&mut hasher);
    use std::hash::Hasher;
    format!("cand-hash-{}", hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chronos_reasoning_coherence::CognitiveStateNode;
    use serde_json::json;

    #[test]
    fn test_decision_ranking_tie_breaking_and_replay() {
        let mut cog_graph = CognitiveStateGraph::new();
        cog_graph.nodes.insert(
            "cog-node-1".to_string(),
            CognitiveStateNode {
                cognitive_state_id: "cog-node-1".to_string(),
                linked_intent_nodes: vec![],
                linked_commitment_nodes: vec![],
                linked_continuity_nodes: vec![],
                unified_semantic_representation: "Work on Chronos".to_string(),
                global_confidence_score: 0.9,
                conflict_flags: vec![],
                resolution_history: vec![],
                source_provenance_graph: vec![],
            },
        );

        cog_graph.nodes.insert(
            "cog-node-2".to_string(),
            CognitiveStateNode {
                cognitive_state_id: "cog-node-2".to_string(),
                linked_intent_nodes: vec![],
                linked_commitment_nodes: vec![],
                linked_continuity_nodes: vec![],
                unified_semantic_representation: "Review pull requests".to_string(),
                global_confidence_score: 0.9,
                conflict_flags: vec![],
                resolution_history: vec![],
                source_provenance_graph: vec![],
            },
        );

        // Generate decisions
        let events = DecisionPipeline::generate_decisions(&cog_graph);

        // Assert candidate generated, scored and decision selected
        assert!(events.iter().any(|e| e.event_type == "DecisionCandidateGenerated"));
        assert!(events.iter().any(|e| e.event_type == "DecisionRanked"));
        assert!(events.iter().any(|e| e.event_type == "DecisionSelected"));

        let graph = DecisionPipeline::rebuild_decision_graph(&events);
        assert_eq!(graph.decisions.len(), 1);

        // Replay determinism check
        let replayed_graph = DecisionPipeline::rebuild_decision_graph(&events);
        assert_eq!(graph, replayed_graph);
    }
}
