//! # Chronos Context Continuation Engine (CCE)
//!
//! Transforms formal decision outputs (ChronosDecision) into concrete, actionable
//! continuation plans (ChronosAction) such as Workspace Restores or Recovery Plans.

use chronos_core::{ChronosAction, ChronosDecision, ChronosEvent};
use chronos_memory_sessions::SessionProjection;
use chronos_reasoning_commitments::CommitmentCandidate;
use chronos_reasoning_dde::DeadlineCandidate;
use chronos_reasoning_risk::RiskForecast;
use serde_json::json;

pub struct CceEngine;

impl CceEngine {
    /// Translates a ChronosDecision into an executable ChronosAction plan.
    pub fn translate_decision(
        decision: &ChronosDecision,
        _state: &chronos_core::ChronosState,
        sessions: &SessionProjection,
        commitments: &[CommitmentCandidate],
        _deadlines: &[DeadlineCandidate],
        _risk: &RiskForecast,
    ) -> Option<ChronosAction> {
        match decision.action_type.as_str() {
            "SuggestWorkspaceRestore" => {
                // Determine target session & files to reopen from last active session
                let last_session = sessions.latest();
                let last_session_id = last_session.map(|s| s.session_id.clone()).unwrap_or_default();
                let mut files = Vec::new();
                if let Some(sess) = last_session {
                    files.extend(sess.artifact_ids.clone());
                }

                Some(ChronosAction::new(
                    decision.id.clone(),
                    "WorkspaceRestoreRequest",
                    json!({
                        "restore_target_session_id": last_session_id,
                        "files_to_reopen": files,
                        "explanation": decision.explanation,
                    }),
                ))
            }
            "SuggestRecoveryPlan" => {
                // Determine re-entry points (files linked to commitments)
                let mut re_entry = Vec::new();
                let mut trajectories = serde_json::Map::new();
                
                for c in commitments {
                    re_entry.extend(c.originating_entities.clone());
                    trajectories.insert(c.commitment_id.clone(), json!("stalled-recovery"));
                }

                Some(ChronosAction::new(
                    decision.id.clone(),
                    "RecoveryPlan",
                    json!({
                        "project_recovery_trajectories": serde_json::Value::Object(trajectories),
                        "dormant_project_reentry_points": re_entry,
                        "recommended_next_action": "Resume development on stalled commitments",
                    }),
                ))
            }
            "Notify" | "EscalateIntervention" => {
                Some(ChronosAction::new(
                    decision.id.clone(),
                    "ContinuationPlan",
                    json!({
                        "digest": decision.explanation,
                        "recommended_next_action": "Review close deadlines",
                    }),
                ))
            }
            _ => None,
        }
    }

    /// Converts a ChronosAction into a COM-compatible event log.
    pub fn to_event(action: &ChronosAction) -> ChronosEvent {
        let event_type = match action.action_type.as_str() {
            "WorkspaceRestoreRequest" => "WorkspaceRestoreRequested",
            "RecoveryPlan" => "RecoveryPlanResolved",
            _ => "ContinuationPlanResolved",
        };

        ChronosEvent::new(
            event_type,
            "CceEngine",
            serde_json::json!({
                "action_id": action.id,
                "decision_id": action.decision_id,
                "action_type": action.action_type,
                "payload": action.payload,
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
    use chronos_decision_orchestrator::DecisionOrchestrator;
    use serde_json::json;

    #[test]
    fn test_action_translation() {
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

        let action = CceEngine::translate_decision(
            &decision,
            &state,
            session_engine.projection(),
            &commitments,
            &deadlines,
            &forecast,
        );

        // NoAction decision type yields None
        assert!(action.is_none());
    }

    #[test]
    fn test_workspace_restore_action() {
        // Create an explicit SuggestWorkspaceRestore decision
        let decision = ChronosDecision::new(
            90,
            "restore suggest",
            vec![],
            "SuggestWorkspaceRestore",
            json!({}),
            None,
        );

        let mut resolver = EntityResolver::new();
        let disc = ChronosEvent::new("GitRepositoryDiscovered", "Git", json!({ "repository_path": "/workspace" }));
        resolver.process_event(&disc).unwrap();
        let graph = resolver.graph().clone();
        let mut session_engine = chronos_memory_sessions::SessionEngine::new(10, graph.clone());
        session_engine.process_event(&disc).unwrap();
        let state = StateProjector::project(&graph, session_engine.projection());

        let action = CceEngine::translate_decision(
            &decision,
            &state,
            session_engine.projection(),
            &[],
            &[],
            &RiskForecast {
                project_failure_probabilities: std::collections::HashMap::new(),
                context_decay_trajectory: std::collections::HashMap::new(),
                intervention_urgency: 0.0,
                confidence: 0.9,
                provenance_ids: vec![],
            },
        ).unwrap();

        assert_eq!(action.action_type, "WorkspaceRestoreRequest");
        assert!(action.payload.get("restore_target_session_id").is_some());
    }
}
