//! # Chronos Risk Analysis Engine
//!
//! Evaluates project capacity, near deadlines, and inactive periods to forecast
//! project failure probabilities, context decay trajectories, and intervention urgencies.

use chrono::{DateTime, Utc};
use chronos_core::ChronosEvent;
use chronos_memory_sessions::SessionProjection;
use chronos_reasoning_commitments::CommitmentCandidate;
use chronos_reasoning_dde::DeadlineCandidate;
use chronos_reasoning_pcm::CapacityProfile;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Details project failure probability, context decay, and urgent intervention alerts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskForecast {
    pub project_failure_probabilities: HashMap<String, f64>,
    pub context_decay_trajectory: HashMap<String, f64>,
    pub intervention_urgency: f64,
    pub confidence: f64,
    pub provenance_ids: Vec<String>,
}

pub struct RiskEngine;

impl RiskEngine {
    /// Computes the RiskForecast from state projections and reasoning outputs.
    pub fn calculate_risk(
        _state: &chronos_core::ChronosState,
        sessions: &SessionProjection,
        commitments: &[CommitmentCandidate],
        deadlines: &[DeadlineCandidate],
        capacity: &CapacityProfile,
    ) -> RiskForecast {
        let mut provenance_ids = capacity.provenance_ids.clone();
        let mut project_failure_probabilities = HashMap::new();
        let mut context_decay_trajectory = HashMap::new();

        // Group commitments and deadlines by Project ID
        let mut project_commitments: HashMap<String, Vec<&CommitmentCandidate>> = HashMap::new();
        for c in commitments {
            for entity_id in &c.originating_entities {
                project_commitments.entry(entity_id.clone()).or_default().push(c);
            }
        }

        let mut project_deadlines: HashMap<String, Vec<&DeadlineCandidate>> = HashMap::new();
        for d in deadlines {
            project_deadlines.entry(d.commitment_id.clone()).or_default().push(d);
        }

        // Calculate Project Failure Probability & Context Decay
        for project_id in project_commitments.keys() {
            let mut near_deadline = false;
            if let Some(deads) = project_deadlines.get(project_id) {
                for d in deads {
                    provenance_ids.push(d.deadline_id.clone());
                    let diff = d.target_date - Utc::now();
                    if diff.num_days() >= 0 && diff.num_days() < 3 {
                        near_deadline = true;
                    }
                }
            }

            // Failure probability formula: baseline 0.10 + capacity weight + burnout weight + deadline weight
            let mut prob = 0.10;
            prob += (1.0 - capacity.throughput_score) * 0.40;
            prob += capacity.burnout_risk * 0.30;
            if near_deadline {
                prob += 0.20;
            }
            let final_prob = prob.max(0.0).min(1.0);
            project_failure_probabilities.insert(project_id.clone(), final_prob);

            // Context Decay: based on hours elapsed since last session
            let mut last_session_time: Option<DateTime<Utc>> = None;
            for session in sessions.sessions().values() {
                if session.project_ids.contains(project_id) {
                    if let Some(t) = last_session_time {
                        if session.end_timestamp > t {
                            last_session_time = Some(session.end_timestamp);
                        }
                    } else {
                        last_session_time = Some(session.end_timestamp);
                    }
                }
            }

            let decay = if let Some(t) = last_session_time {
                let elapsed_hours = (Utc::now() - t).num_seconds() as f64 / 3600.0;
                (elapsed_hours / 72.0).min(1.0).max(0.0)
            } else {
                1.0 // total decay if no session recorded
            };
            context_decay_trajectory.insert(project_id.clone(), decay);
        }

        // Intervention Urgency = max(burnout_risk, max(failure_prob))
        let max_fail = project_failure_probabilities.values()
            .cloned()
            .fold(0.0, f64::max);

        let intervention_urgency = capacity.burnout_risk.max(max_fail);

        RiskForecast {
            project_failure_probabilities,
            context_decay_trajectory,
            intervention_urgency,
            confidence: 0.90,
            provenance_ids,
        }
    }

    /// Converts a RiskForecast structure into a COM-compatible Event.
    pub fn to_event(forecast: &RiskForecast) -> ChronosEvent {
        ChronosEvent::new(
            "RiskForecastResolved",
            "RiskEngine",
            serde_json::to_value(forecast).unwrap_or(serde_json::Value::Null),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chronos_memory_entity_resolution::EntityResolver;
    use chronos_memory_state::StateProjector;
    use chronos_reasoning_pcm::CapacityEngine;
    use serde_json::json;

    #[test]
    fn test_risk_forecasting() {
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

        assert!(forecast.intervention_urgency >= 0.0);
        assert_eq!(forecast.confidence, 0.90);
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

        let f1 = RiskEngine::calculate_risk(&state, session_engine.projection(), &commitments, &deadlines, &capacity);
        let f2 = RiskEngine::calculate_risk(&state, session_engine.projection(), &commitments, &deadlines, &capacity);

        assert_eq!(f1.intervention_urgency, f2.intervention_urgency);
        assert_eq!(f1.project_failure_probabilities, f2.project_failure_probabilities);
    }
}
