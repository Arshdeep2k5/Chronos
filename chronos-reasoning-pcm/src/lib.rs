//! # Chronos Personal Capacity Model (PCM) Engine
//!
//! Estimates available focus, stability, throughput, and burnout risk metrics
//! deterministically from user history.

use chronos_core::ChronosEvent;
use chronos_memory_sessions::SessionProjection;
use chronos_reasoning_commitments::CommitmentCandidate;
use chronos_reasoning_dde::DeadlineCandidate;
use serde::{Deserialize, Serialize};

/// Represents a capacity profile snapshot detailing focus performance and workload risks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapacityProfile {
    pub capacity_score: f64,
    pub focus_score: f64,
    pub throughput_score: f64,
    pub stability_score: f64,
    pub burnout_risk: f64,
    pub confidence: f64,
    pub provenance_ids: Vec<String>,
}

pub struct CapacityEngine;

impl CapacityEngine {
    /// Computes the Personal Capacity Profile from state projections and reasoning outputs.
    pub fn estimate_capacity(
        _state: &chronos_core::ChronosState,
        sessions: &SessionProjection,
        commitments: &[CommitmentCandidate],
        deadlines: &[DeadlineCandidate],
    ) -> CapacityProfile {
        let mut provenance_ids = Vec::new();

        // 1. Session Velocity & Focus calculation
        let session_count = sessions.sessions().len() as f64;
        let mut total_duration = 0.0;
        let mut repos_touched = std::collections::HashSet::new();

        for session in sessions.sessions().values() {
            total_duration += session.duration as f64;
            provenance_ids.push(session.session_id.clone());
            for r_id in &session.repository_ids {
                repos_touched.insert(r_id.clone());
            }
        }

        let average_session_duration = if session_count > 0.0 {
            total_duration / session_count
        } else {
            0.0
        };

        // 2. Commitment throughput
        let commitments_created = commitments.len() as f64;
        for c in commitments {
            provenance_ids.push(c.commitment_id.clone());
        }
        for d in deadlines {
            provenance_ids.push(d.deadline_id.clone());
        }

        // Context switching frequency = unique repos / sessions (capped at 5)
        let context_switching_frequency = if session_count > 0.0 {
            repos_touched.len() as f64 / session_count
        } else {
            0.0
        };

        // 3. Compute metric scores
        let focus_score = (average_session_duration / 3600.0).min(1.0);
        let throughput_score = (commitments_created / 10.0).min(1.0);
        let stability_score = (1.0 - (context_switching_frequency / 5.0)).max(0.0).min(1.0);

        // sessions per day estimate (using a fallback of 1 if history is sparse)
        let sessions_per_day = if session_count > 0.0 {
            session_count / 1.0
        } else {
            0.0
        };

        // 4. Burnout risk
        let burnout_risk = if sessions_per_day > 8.0 || average_session_duration > 7200.0 {
            0.85
        } else {
            0.10
        };

        // 5. Total Capacity Score
        let capacity_score = ((focus_score + throughput_score + stability_score) / 3.0 * (1.0 - burnout_risk))
            .max(0.0)
            .min(1.0);

        CapacityProfile {
            capacity_score,
            focus_score,
            throughput_score,
            stability_score,
            burnout_risk,
            confidence: 0.90, // baseline certainty
            provenance_ids,
        }
    }

    /// Converts a CapacityProfile structure into a COM-compatible Event.
    pub fn to_event(profile: &CapacityProfile) -> ChronosEvent {
        ChronosEvent::new(
            "CapacityProfileResolved",
            "CapacityEngine",
            serde_json::to_value(profile).unwrap_or(serde_json::Value::Null),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chronos_memory_entity_resolution::EntityResolver;
    use chronos_memory_state::StateProjector;
    use serde_json::json;

    #[test]
    fn test_capacity_estimation() {
        let mut resolver = EntityResolver::new();
        let disc = ChronosEvent::new("GitRepositoryDiscovered", "Git", json!({ "repository_path": "/workspace" }));
        resolver.process_event(&disc).unwrap();

        let graph = resolver.graph().clone();
        let mut session_engine = chronos_memory_sessions::SessionEngine::new(10, graph.clone());
        session_engine.process_event(&disc).unwrap();

        let state = StateProjector::project(&graph, session_engine.projection());
        
        let commitments = chronos_reasoning_commitments::CommitmentEngine::resolve_commitments(&state, &graph, session_engine.projection());
        let deadlines = chronos_reasoning_dde::DeadlineEngine::discover_deadlines(&state, &commitments, session_engine.projection(), &graph, &[]);

        let profile = CapacityEngine::estimate_capacity(&state, session_engine.projection(), &commitments, &deadlines);

        assert!(profile.capacity_score >= 0.0);
        assert_eq!(profile.confidence, 0.90);
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

        let p1 = CapacityEngine::estimate_capacity(&state, session_engine.projection(), &commitments, &deadlines);
        let p2 = CapacityEngine::estimate_capacity(&state, session_engine.projection(), &commitments, &deadlines);

        assert_eq!(p1.capacity_score, p2.capacity_score);
        assert_eq!(p1.burnout_risk, p2.burnout_risk);
    }
}
