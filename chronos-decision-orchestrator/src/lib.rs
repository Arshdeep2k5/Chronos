//! # Chronos Decision Orchestrator
//!
//! Evaluates states, risks, workloads, and reflections to resolve deterministic
//! recovery, notify, and intervention decisions (ChronosDecision).


use chronos_core::ChronosDecision;
use chronos_memory_sessions::SessionProjection;
use chronos_reasoning_commitments::CommitmentCandidate;
use chronos_reasoning_dde::DeadlineCandidate;
use chronos_reasoning_pcm::CapacityProfile;
use chronos_reasoning_risk::RiskForecast;
use serde::{Deserialize, Serialize};

/// Detailed category type of the resolved decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DecisionType {
    NoAction,
    Notify,
    SuggestRecoveryPlan,
    SuggestWorkspaceRestore,
    EscalateIntervention,
    SuppressIntervention,
}

impl DecisionType {
    pub fn as_str(&self) -> &'static str {
        match self {
            DecisionType::NoAction => "NoAction",
            DecisionType::Notify => "Notify",
            DecisionType::SuggestRecoveryPlan => "SuggestRecoveryPlan",
            DecisionType::SuggestWorkspaceRestore => "SuggestWorkspaceRestore",
            DecisionType::EscalateIntervention => "EscalateIntervention",
            DecisionType::SuppressIntervention => "SuppressIntervention",
        }
    }
}

/// The structured payload serialized inside the action_payload field of ChronosDecision.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratedDecisionPayload {
    pub silence_cost: f64,
    pub interruption_cost: f64,
    pub detailed_explanation: String,
}

pub struct DecisionOrchestrator;

impl DecisionOrchestrator {
    /// Arbitrates multiple reasoning signals to resolve a single deterministic ChronosDecision.
    pub fn orchestrate_decision(
        _state: &chronos_core::ChronosState,
        forecast: &RiskForecast,
        capacity: &CapacityProfile,
        _commitments: &[CommitmentCandidate],
        _deadlines: &[DeadlineCandidate],
        _reflections: &[chronos_core::ChronosReflection],
        sessions: &SessionProjection,
    ) -> ChronosDecision {
        let mut provenance_ids = forecast.provenance_ids.clone();
        
        // 1. Calculate Silence and Interruption Costs
        let max_failure_prob = forecast.project_failure_probabilities.values()
            .cloned()
            .fold(0.0, f64::max);
        let silence_cost = max_failure_prob;
        let interruption_cost = capacity.burnout_risk;

        // 2. Resolve Decision Type using Arbitration Matrix
        let mut decision_type = DecisionType::NoAction;
        let mut explanation = "No significant risk patterns resolved; maintaining passive watch.".to_string();

        if max_failure_prob > 0.75 {
            decision_type = DecisionType::EscalateIntervention;
            explanation = format!("Critical risk alert: Project failure probability reached {:.2}.", max_failure_prob);
        } else if interruption_cost > 0.80 {
            decision_type = DecisionType::SuppressIntervention;
            explanation = format!("Burnout risk is elevated ({:.2}); suppressing alerts to preserve focus.", interruption_cost);
        } else {
            let has_interrupted_session = sessions.sessions().values().any(|s| s.is_closed && s.duration > 300);
            
            if has_interrupted_session {
                decision_type = DecisionType::SuggestWorkspaceRestore;
                explanation = "Inactivity timeout detected on last active session. Suggesting workspace restore.".to_string();
            } else if max_failure_prob > 0.40 {
                decision_type = DecisionType::SuggestRecoveryPlan;
                explanation = "Moderate workload decay detected. Suggesting capacity recovery plan.".to_string();
            } else if max_failure_prob > 0.15 {
                decision_type = DecisionType::Notify;
                explanation = "Approaching milestones noted; sending routine status digest.".to_string();
            }
        }

        let confidence_score = (capacity.confidence + forecast.confidence) / 2.0;

        let payload = OrchestratedDecisionPayload {
            silence_cost,
            interruption_cost,
            detailed_explanation: explanation.clone(),
        };

        provenance_ids.sort();
        provenance_ids.dedup();

        // Create the core ChronosDecision
        ChronosDecision::new(
            (confidence_score * 100.0) as u8,
            explanation,
            provenance_ids,
            decision_type.as_str(),
            serde_json::to_value(&payload).unwrap_or(serde_json::Value::Null),
            None, // no default expiry
        )
    }

    /// Converts a ChronosDecision structure into a COM-compatible Event.
    pub fn to_event(decision: &ChronosDecision) -> chronos_core::ChronosEvent {
        chronos_core::ChronosEvent::new(
            "DecisionResolved",
            "DecisionOrchestrator",
            serde_json::json!({
                "decision_id": decision.id,
                "timestamp": decision.timestamp.to_rfc3339(),
                "confidence": decision.confidence,
                "explanation": decision.explanation,
                "evidence_ids": decision.evidence_ids,
                "action_type": decision.action_type,
                "action_payload": decision.action_payload,
            }),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chronos_core::ChronosEvent;
    use chronos_memory_entity_resolution::EntityResolver;
    use chronos_memory_state::StateProjector;
    use chronos_reasoning_pcm::CapacityEngine;
    use chronos_reasoning_risk::RiskEngine;
    use serde_json::json;

    #[test]
    fn test_decision_orchestration() {
        let mut resolver = EntityResolver::new();
        let disc = ChronosEvent::new("GitRepositoryDiscovered", "Git", json!({ "repository_path": "/workspace" }));
        resolver.process_event(&disc).unwrap();

        let graph = resolver.graph().clone();
        let mut session_engine = chronos_memory_sessions::SessionEngine::new(10, graph.clone());
        session_engine.process_event(&disc).unwrap();

        let state = StateProjector::project(&graph, session_engine.projection());
        
        let commitments = chronos_reasoning_commitments::CommitmentEngine::resolve_commitments(&state, &graph, session_engine.projection());
        let deadlines = chronos_reasoning_dde::DeadlineEngine::discover_deadlines(&state, &commitments, session_engine.projection(), &graph, &[]);
        let capacity = CapacityEngine::estimate_capacity(&state, session_engine.projection(), &commitments, &deadlines);
        let forecast = RiskEngine::calculate_risk(&state, session_engine.projection(), &commitments, &deadlines, &capacity);

        let decision = DecisionOrchestrator::orchestrate_decision(
            &state,
            &forecast,
            &capacity,
            &commitments,
            &deadlines,
            &[],
            session_engine.projection(),
        );

        assert!(decision.confidence > 0);
        assert!(decision.explanation.len() > 0);
        assert_eq!(decision.action_type, "NoAction");
    }

    #[test]
    fn test_replay_determinism() {
        let mut resolver = EntityResolver::new();
        let disc = ChronosEvent::new("GitRepositoryDiscovered", "Git", json!({ "repository_path": "/workspace" }));
        resolver.process_event(&disc).unwrap();

        let graph = resolver.graph().clone();
        let mut session_engine = chronos_memory_sessions::SessionEngine::new(10, graph.clone());
        session_engine.process_event(&disc).unwrap();

        let state = StateProjector::project(&graph, session_engine.projection());
        
        let commitments = chronos_reasoning_commitments::CommitmentEngine::resolve_commitments(&state, &graph, session_engine.projection());
        let deadlines = chronos_reasoning_dde::DeadlineEngine::discover_deadlines(&state, &commitments, session_engine.projection(), &graph, &[]);
        let capacity = CapacityEngine::estimate_capacity(&state, session_engine.projection(), &commitments, &deadlines);
        let forecast = RiskEngine::calculate_risk(&state, session_engine.projection(), &commitments, &deadlines, &capacity);

        let d1 = DecisionOrchestrator::orchestrate_decision(
            &state,
            &forecast,
            &capacity,
            &commitments,
            &deadlines,
            &[],
            session_engine.projection(),
        );
        let d2 = DecisionOrchestrator::orchestrate_decision(
            &state,
            &forecast,
            &capacity,
            &commitments,
            &deadlines,
            &[],
            session_engine.projection(),
        );

        assert_eq!(d1.action_type, d2.action_type);
        assert_eq!(d1.explanation, d2.explanation);
        assert_eq!(d1.confidence, d2.confidence);
    }
}
