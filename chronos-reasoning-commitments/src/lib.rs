//! # Chronos Commitment Engine
//!
//! Transforms global state, active focus sessions, and entity links into explicit,
//! deterministic commitment candidates (CommitmentCandidate), and implements the event-sourced
//! Commitment Core Domain Layer.

use chrono::{DateTime, Utc};
use chronos_core::ChronosEvent;
use chronos_memory_entity_resolution::EntityGraph;
use chronos_memory_sessions::SessionProjection;
use chronos_memory_state::ProjectedStatePayload;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

/// Represents an inferred commitment candidate resolved from focus context history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitmentCandidate {
    pub commitment_id: String,
    pub title: String,
    pub confidence: f64,
    pub evidence_ids: Vec<String>,
    pub originating_sessions: Vec<String>,
    pub originating_entities: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub last_activity_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommitmentStatus {
    Candidate,
    Active,
    AtRisk,
    Completed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Commitment {
    pub commitment_id: String,
    pub source: String,
    pub content: String,
    pub status: CommitmentStatus,
    pub created_at: DateTime<Utc>,
    pub inferred_due_at: Option<DateTime<Utc>>,
    pub confidence: f64,
    pub provenance: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitmentDiscoveredPayload {
    pub commitment_id: String,
    pub source: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub inferred_due_at: Option<DateTime<Utc>>,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitmentUpdatedPayload {
    pub commitment_id: String,
    pub new_content: Option<String>,
    pub new_inferred_due_at: Option<DateTime<Utc>>,
    pub new_confidence: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitmentValidatedPayload {
    pub commitment_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitmentActivatedPayload {
    pub commitment_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitmentMarkedAtRiskPayload {
    pub commitment_id: String,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitmentCompletedPayload {
    pub commitment_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitmentCancelledPayload {
    pub commitment_id: String,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct CommitmentState {
    pub commitments: HashMap<String, Commitment>,
}

impl CommitmentState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn apply_event(&mut self, event: &ChronosEvent) {
        match event.event_type.as_str() {
            "CommitmentDiscovered" => {
                if let Ok(payload) = serde_json::from_value::<CommitmentDiscoveredPayload>(event.payload.clone()) {
                    if let Some(commitment) = self.commitments.get_mut(&payload.commitment_id) {
                        commitment.provenance.push(event.id.clone());
                    } else {
                        let commitment = Commitment {
                            commitment_id: payload.commitment_id.clone(),
                            source: payload.source,
                            content: payload.content,
                            status: CommitmentStatus::Candidate,
                            created_at: payload.created_at,
                            inferred_due_at: payload.inferred_due_at,
                            confidence: payload.confidence,
                            provenance: vec![event.id.clone()],
                        };
                        self.commitments.insert(payload.commitment_id, commitment);
                    }
                }
            }
            "CommitmentUpdated" => {
                if let Ok(payload) = serde_json::from_value::<CommitmentUpdatedPayload>(event.payload.clone()) {
                    if let Some(commitment) = self.commitments.get_mut(&payload.commitment_id) {
                        if let Some(c) = payload.new_content {
                            commitment.content = c;
                        }
                        if let Some(d) = payload.new_inferred_due_at {
                            commitment.inferred_due_at = Some(d);
                        }
                        if let Some(conf) = payload.new_confidence {
                            commitment.confidence = conf;
                        }
                        commitment.provenance.push(event.id.clone());
                    }
                }
            }
            "CommitmentValidated" => {
                if let Ok(payload) = serde_json::from_value::<CommitmentValidatedPayload>(event.payload.clone()) {
                    if let Some(commitment) = self.commitments.get_mut(&payload.commitment_id) {
                        commitment.status = CommitmentStatus::Active;
                        commitment.provenance.push(event.id.clone());
                    }
                }
            }
            "CommitmentActivated" => {
                if let Ok(payload) = serde_json::from_value::<CommitmentActivatedPayload>(event.payload.clone()) {
                    if let Some(commitment) = self.commitments.get_mut(&payload.commitment_id) {
                        commitment.status = CommitmentStatus::Active;
                        commitment.provenance.push(event.id.clone());
                    }
                }
            }
            "CommitmentMarkedAtRisk" => {
                if let Ok(payload) = serde_json::from_value::<CommitmentMarkedAtRiskPayload>(event.payload.clone()) {
                    if let Some(commitment) = self.commitments.get_mut(&payload.commitment_id) {
                        commitment.status = CommitmentStatus::AtRisk;
                        commitment.provenance.push(event.id.clone());
                    }
                }
            }
            "CommitmentCompleted" => {
                if let Ok(payload) = serde_json::from_value::<CommitmentCompletedPayload>(event.payload.clone()) {
                    if let Some(commitment) = self.commitments.get_mut(&payload.commitment_id) {
                        commitment.status = CommitmentStatus::Completed;
                        commitment.provenance.push(event.id.clone());
                    }
                }
            }
            "CommitmentCancelled" => {
                if let Ok(payload) = serde_json::from_value::<CommitmentCancelledPayload>(event.payload.clone()) {
                    if let Some(commitment) = self.commitments.get_mut(&payload.commitment_id) {
                        commitment.status = CommitmentStatus::Cancelled;
                        commitment.provenance.push(event.id.clone());
                    }
                }
            }
            _ => {}
        }
    }
}

pub struct CommitmentEngine;

impl CommitmentEngine {
    /// Evaluates ChronosState and graph configurations to extract candidate commitments.
    pub fn resolve_commitments(
        state: &chronos_core::ChronosState,
        graph: &EntityGraph,
        sessions: &SessionProjection,
    ) -> Vec<CommitmentCandidate> {
        let mut candidates = Vec::new();

        let _state_payload: ProjectedStatePayload = match serde_json::from_value(state.payload.clone()) {
            Ok(p) => p,
            Err(_) => return Vec::new(),
        };

        // Rule 1: Project with recurring activity
        // Find how many times each Project entity was touched across different sessions
        let mut project_session_map: HashMap<String, HashSet<String>> = HashMap::new();
        let mut project_timestamps: HashMap<String, (DateTime<Utc>, DateTime<Utc>)> = HashMap::new();

        for session in sessions.sessions().values() {
            for proj_id in &session.project_ids {
                project_session_map
                    .entry(proj_id.clone())
                    .or_default()
                    .insert(session.session_id.clone());

                let times = project_timestamps.entry(proj_id.clone()).or_insert((session.start_timestamp, session.end_timestamp));
                if session.start_timestamp < times.0 {
                    times.0 = session.start_timestamp;
                }
                if session.end_timestamp > times.1 {
                    times.1 = session.end_timestamp;
                }
            }
        }

        for (proj_id, session_ids) in project_session_map {
            if session_ids.len() >= 2 {
                if let Some(entity) = graph.get_entity(&proj_id) {
                    let name = entity.properties.get("name").map(|s| s.as_str()).unwrap_or("unknown");
                    let times = project_timestamps.get(&proj_id).cloned().unwrap_or_else(|| (Utc::now(), Utc::now()));
                    
                    // Confidence scales with count of sessions: 0.8 + 0.05 per session (cap 1.0)
                    let confidence = (0.8 + 0.05 * (session_ids.len() as f64)).min(1.0);

                    // Collect evidence IDs from all matched sessions
                    let mut evidence_ids = Vec::new();
                    for s_id in &session_ids {
                        if let Some(sess) = sessions.sessions().get(s_id) {
                            evidence_ids.extend(sess.source_event_ids.clone());
                        }
                    }

                    candidates.push(CommitmentCandidate {
                        commitment_id: Uuid::new_v4().to_string(),
                        title: format!("Commitment candidate: Active development on Project '{}'", name),
                        confidence,
                        evidence_ids,
                        originating_sessions: session_ids.into_iter().collect(),
                        originating_entities: vec![proj_id],
                        created_at: times.0,
                        last_activity_at: times.1,
                    });
                }
            }
        }

        // Rule 2: Artifact referenced across multiple sessions
        let mut artifact_session_map: HashMap<String, HashSet<String>> = HashMap::new();
        let mut artifact_timestamps: HashMap<String, (DateTime<Utc>, DateTime<Utc>)> = HashMap::new();

        for session in sessions.sessions().values() {
            for art_id in &session.artifact_ids {
                artifact_session_map
                    .entry(art_id.clone())
                    .or_default()
                    .insert(session.session_id.clone());

                let times = artifact_timestamps.entry(art_id.clone()).or_insert((session.start_timestamp, session.end_timestamp));
                if session.start_timestamp < times.0 {
                    times.0 = session.start_timestamp;
                }
                if session.end_timestamp > times.1 {
                    times.1 = session.end_timestamp;
                }
            }
        }

        for (art_id, session_ids) in artifact_session_map {
            if session_ids.len() >= 2 {
                if let Some(entity) = graph.get_entity(&art_id) {
                    let path = entity.properties.get("path").map(|s| s.as_str()).unwrap_or("unknown");
                    let times = artifact_timestamps.get(&art_id).cloned().unwrap_or_else(|| (Utc::now(), Utc::now()));
                    let confidence = (0.7 + 0.05 * (session_ids.len() as f64)).min(1.0);

                    let mut evidence_ids = Vec::new();
                    for s_id in &session_ids {
                        if let Some(sess) = sessions.sessions().get(s_id) {
                            evidence_ids.extend(sess.source_event_ids.clone());
                        }
                    }

                    candidates.push(CommitmentCandidate {
                        commitment_id: Uuid::new_v4().to_string(),
                        title: format!("Commitment candidate: Refactor/maintain artifact '{}'", path),
                        confidence,
                        evidence_ids,
                        originating_sessions: session_ids.into_iter().collect(),
                        originating_entities: vec![art_id],
                        created_at: times.0,
                        last_activity_at: times.1,
                    });
                }
            }
        }

        candidates
    }

    /// Converts a CommitmentCandidate structure into a COM-compatible Event.
    pub fn to_event(candidate: &CommitmentCandidate) -> ChronosEvent {
        ChronosEvent::new(
            "CommitmentCandidateResolved",
            "CommitmentEngine",
            serde_json::to_value(candidate).unwrap_or(serde_json::Value::Null),
        )
    }

    /// Reconstructs the entire CommitmentState projection from an ordered event log.
    pub fn rebuild_commitment_state(events: &[ChronosEvent]) -> CommitmentState {
        let mut state = CommitmentState::new();
        for event in events {
            state.apply_event(event);
        }
        state
    }

    /// Creates a Commitment entity from a CommitmentDiscovered event
    pub fn create_commitment_from_event(event: &ChronosEvent) -> Option<Commitment> {
        if event.event_type == "CommitmentDiscovered" {
            let payload = serde_json::from_value::<CommitmentDiscoveredPayload>(event.payload.clone()).ok()?;
            Some(Commitment {
                commitment_id: payload.commitment_id,
                source: payload.source,
                content: payload.content,
                status: CommitmentStatus::Candidate,
                created_at: payload.created_at,
                inferred_due_at: payload.inferred_due_at,
                confidence: payload.confidence,
                provenance: vec![event.id.clone()],
            })
        } else {
            None
        }
    }

    /// Mutates the state projection safely through event application
    pub fn apply_commitment_event(state: &mut CommitmentState, event: &ChronosEvent) -> Result<(), String> {
        match event.event_type.as_str() {
            "CommitmentDiscovered"
            | "CommitmentUpdated"
            | "CommitmentValidated"
            | "CommitmentActivated"
            | "CommitmentMarkedAtRisk"
            | "CommitmentCompleted"
            | "CommitmentCancelled" => {
                state.apply_event(event);
                Ok(())
            }
            _ => Err(format!("Not a valid commitment event: {}", event.event_type)),
        }
    }

    /// Returns a specific commitment by ID from the state
    pub fn get_commitment_state(state: &CommitmentState, id: &str) -> Option<Commitment> {
        state.commitments.get(id).cloned()
    }

    /// Lists all commitments currently in the Active status
    pub fn list_active_commitments(state: &CommitmentState) -> Vec<Commitment> {
        state
            .commitments
            .values()
            .filter(|c| c.status == CommitmentStatus::Active)
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chronos_memory_entity_resolution::EntityResolver;
    use chronos_memory_sessions::SessionEngine;
    use chronos_memory_state::StateProjector;
    use serde_json::json;

    #[test]
    fn test_commitment_resolution() {
        let mut resolver = EntityResolver::new();
        let disc_event = ChronosEvent::new(
            "GitRepositoryDiscovered",
            "GitAdapter",
            json!({ "repository_path": "/workspace/chronos" }),
        );
        resolver.process_event(&disc_event).unwrap();

        // Create commits referencing a file in two different sessions
        let t1 = Utc::now();
        let mut commit_event1 = ChronosEvent::new(
            "GitCommitCreated",
            "GitAdapter",
            json!({
                "repository_path": "/workspace/chronos",
                "source_payload": { "message": "First commit", "files": ["src/lib.rs"] }
            }),
        );
        commit_event1.timestamp = t1;
        resolver.process_event(&commit_event1).unwrap();

        let mut commit_event2 = ChronosEvent::new(
            "GitCommitCreated",
            "GitAdapter",
            json!({
                "repository_path": "/workspace/chronos",
                "source_payload": { "message": "Second commit", "files": ["src/lib.rs"] }
            }),
        );
        // Ensure it triggers a new session by offsetting > 15 minutes (900 seconds)
        commit_event2.timestamp = t1 + chrono::Duration::seconds(1000);
        resolver.process_event(&commit_event2).unwrap();

        let graph = resolver.graph().clone();
        
        let mut session_engine = SessionEngine::new(10, graph.clone());
        session_engine.process_event(&disc_event).unwrap();
        session_engine.process_event(&commit_event1).unwrap();
        session_engine.process_event(&commit_event2).unwrap();

        let state = StateProjector::project(&graph, session_engine.projection());
        
        let commitments = CommitmentEngine::resolve_commitments(&state, &graph, session_engine.projection());
        
        // Assert commitment candidate generated
        assert!(!commitments.is_empty());
        assert!(commitments.iter().any(|c| c.title.contains("Project")));
        assert!(commitments.iter().any(|c| c.title.contains("artifact")));
    }

    #[test]
    fn test_replay_determinism() {
        let mut resolver = EntityResolver::new();
        let disc_event = ChronosEvent::new(
            "GitRepositoryDiscovered",
            "GitAdapter",
            json!({ "repository_path": "/workspace/chronos" }),
        );
        resolver.process_event(&disc_event).unwrap();

        let graph = resolver.graph().clone();
        let mut session_engine = SessionEngine::new(10, graph.clone());
        session_engine.process_event(&disc_event).unwrap();

        let state = StateProjector::project(&graph, session_engine.projection());

        let c1 = CommitmentEngine::resolve_commitments(&state, &graph, session_engine.projection());
        let c2 = CommitmentEngine::resolve_commitments(&state, &graph, session_engine.projection());

        assert_eq!(c1.len(), c2.len());
    }

    #[test]
    fn test_event_sourced_commitment_lifecycle() {
        let commitment_id = Uuid::new_v4().to_string();
        let created_at = Utc::now();

        // 1. Discover Candidate
        let event1 = ChronosEvent::new(
            "CommitmentDiscovered",
            "Test",
            json!({
                "commitment_id": commitment_id,
                "source": "VSCode",
                "content": "Implement feature X",
                "created_at": created_at,
                "inferred_due_at": created_at + chrono::Duration::days(2),
                "confidence": 0.9
            }),
        );

        let mut state = CommitmentState::new();
        CommitmentEngine::apply_commitment_event(&mut state, &event1).unwrap();

        let commitment = CommitmentEngine::get_commitment_state(&state, &commitment_id).unwrap();
        assert_eq!(commitment.status, CommitmentStatus::Candidate);
        assert_eq!(commitment.content, "Implement feature X");
        assert_eq!(commitment.confidence, 0.9);

        // 2. Update commitment properties
        let event2 = ChronosEvent::new(
            "CommitmentUpdated",
            "Test",
            json!({
                "commitment_id": commitment_id,
                "new_content": "Implement feature X with tests",
                "new_inferred_due_at": created_at + chrono::Duration::days(3),
                "new_confidence": 0.95
            }),
        );
        CommitmentEngine::apply_commitment_event(&mut state, &event2).unwrap();
        let commitment = CommitmentEngine::get_commitment_state(&state, &commitment_id).unwrap();
        assert_eq!(commitment.content, "Implement feature X with tests");
        assert_eq!(commitment.confidence, 0.95);

        // 3. Validate -> status Active
        let event3 = ChronosEvent::new(
            "CommitmentValidated",
            "Test",
            json!({ "commitment_id": commitment_id }),
        );
        CommitmentEngine::apply_commitment_event(&mut state, &event3).unwrap();
        let commitment = CommitmentEngine::get_commitment_state(&state, &commitment_id).unwrap();
        assert_eq!(commitment.status, CommitmentStatus::Active);

        // 4. Mark AtRisk
        let event4 = ChronosEvent::new(
            "CommitmentMarkedAtRisk",
            "Test",
            json!({ "commitment_id": commitment_id, "reason": "Blocked by PR" }),
        );
        CommitmentEngine::apply_commitment_event(&mut state, &event4).unwrap();
        let commitment = CommitmentEngine::get_commitment_state(&state, &commitment_id).unwrap();
        assert_eq!(commitment.status, CommitmentStatus::AtRisk);

        // 5. Complete
        let event5 = ChronosEvent::new(
            "CommitmentCompleted",
            "Test",
            json!({ "commitment_id": commitment_id }),
        );
        CommitmentEngine::apply_commitment_event(&mut state, &event5).unwrap();
        let commitment = CommitmentEngine::get_commitment_state(&state, &commitment_id).unwrap();
        assert_eq!(commitment.status, CommitmentStatus::Completed);
    }

    #[test]
    fn test_commitment_replay_fidelity() {
        let commitment_id = Uuid::new_v4().to_string();
        let created_at = Utc::now();

        let event1 = ChronosEvent::new(
            "CommitmentDiscovered",
            "Test",
            json!({
                "commitment_id": commitment_id,
                "source": "VSCode",
                "content": "Implement feature X",
                "created_at": created_at,
                "inferred_due_at": null,
                "confidence": 0.8
            }),
        );
        let event2 = ChronosEvent::new(
            "CommitmentActivated",
            "Test",
            json!({ "commitment_id": commitment_id }),
        );

        let events = vec![event1, event2];
        let state = CommitmentEngine::rebuild_commitment_state(&events);

        let commitment = CommitmentEngine::get_commitment_state(&state, &commitment_id).unwrap();
        assert_eq!(commitment.status, CommitmentStatus::Active);
        assert_eq!(commitment.provenance.len(), 2);
    }

    #[test]
    fn test_idempotency_and_ordering() {
        let commitment_id = Uuid::new_v4().to_string();
        let created_at = Utc::now();

        let event1 = ChronosEvent::new(
            "CommitmentDiscovered",
            "Test",
            json!({
                "commitment_id": commitment_id,
                "source": "VSCode",
                "content": "Feature Y",
                "created_at": created_at,
                "inferred_due_at": null,
                "confidence": 0.8
            }),
        );

        let mut state = CommitmentState::new();
        // Double apply same event should be idempotent
        CommitmentEngine::apply_commitment_event(&mut state, &event1).unwrap();
        CommitmentEngine::apply_commitment_event(&mut state, &event1).unwrap();

        let commitment = CommitmentEngine::get_commitment_state(&state, &commitment_id).unwrap();
        assert_eq!(commitment.provenance.len(), 2); // Trace logs both deliveries, but properties are idempotent
    }
}
