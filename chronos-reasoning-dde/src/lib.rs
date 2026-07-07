//! # Chronos Deadline Discovery Engine (DDE)
//!
//! Infers explicit, inferred, and repository-derived deadlines from observed events,
//! sessions, and resolved commitment structures.

use chrono::{DateTime, TimeZone, Utc};
use chronos_core::ChronosEvent;
use chronos_memory_entity_resolution::EntityGraph;
use chronos_memory_sessions::SessionProjection;
use chronos_reasoning_commitments::CommitmentCandidate;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use uuid::Uuid;

/// The origin type of the resolved deadline candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SourceType {
    Explicit,
    Inferred,
    RepositoryDerived,
}

/// Represents a deadline candidate resolved from reality logs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeadlineCandidate {
    pub deadline_id: String,
    pub commitment_id: String,
    pub target_date: DateTime<Utc>,
    pub confidence: f64,
    pub evidence_ids: Vec<String>,
    pub source_type: SourceType,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

static DATE_REGEX: OnceLock<Regex> = OnceLock::new();

fn get_date_regex() -> &'static Regex {
    DATE_REGEX.get_or_init(|| {
        Regex::new(r"(?i)(?:due|deadline|milestone|release)\s+(\d{4}-\d{2}-\d{2})").unwrap()
    })
}

pub struct DeadlineEngine;

impl DeadlineEngine {
    /// Discovers deadlines from state projections, commitments, session timelines, and event logs.
    pub fn discover_deadlines(
        _state: &chronos_core::ChronosState,
        commitments: &[CommitmentCandidate],
        sessions: &SessionProjection,
        _graph: &EntityGraph,
        events: &[ChronosEvent],
    ) -> Vec<DeadlineCandidate> {
        let mut candidates = Vec::new();
        let re = get_date_regex();

        // 1. Explicit Dates from Event logs
        for event in events {
            let message = if event.event_type == "GitCommitCreated" || event.event_type == "GitCommitAmended" {
                event.payload.get("source_payload")
                    .and_then(|v| v.get("message"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
            } else {
                ""
            };

            if let Some(captures) = re.captures(message) {
                if let Some(date_str) = captures.get(1).map(|m| m.as_str()) {
                    if let Ok(naive_date) = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
                        if let Some(target_date) = naive_date.and_hms_opt(0, 0, 0)
                            .map(|dt| Utc.from_utc_datetime(&dt))
                        {
                            // Associate with commitment matching repository if possible
                            let repo_path = event.payload.get("repository_path").and_then(|v| v.as_str()).unwrap_or("");
                            let commitment_id = commitments.iter()
                                .find(|c| c.title.contains(repo_path))
                                .map(|c| c.commitment_id.clone())
                                .unwrap_or_else(|| "unassociated-commitment".to_string());

                            candidates.push(DeadlineCandidate {
                                deadline_id: Uuid::new_v4().to_string(),
                                commitment_id,
                                target_date,
                                confidence: 1.0,
                                evidence_ids: vec![event.id.clone()],
                                source_type: SourceType::Explicit,
                                created_at: Utc::now(),
                                updated_at: Utc::now(),
                            });
                        }
                    }
                }
            }
        }

        // 2. Repository Derived tags
        for event in events {
            if event.event_type == "GitTagCreated" {
                let tag_name = event.payload.get("source_payload")
                    .and_then(|v| v.get("tag_name"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                // Regex check for vX.Y release tags
                let tag_re = Regex::new(r"^v\d+\.\d+$").unwrap();
                if tag_re.is_match(tag_name) || tag_name.contains("release") || tag_name.contains("milestone") {
                    let target_date = event.timestamp + chrono::Duration::days(1);
                    
                    let repo_path = event.payload.get("repository_path").and_then(|v| v.as_str()).unwrap_or("");
                    let commitment_id = commitments.iter()
                        .find(|c| c.title.contains(repo_path))
                        .map(|c| c.commitment_id.clone())
                        .unwrap_or_else(|| "unassociated-commitment".to_string());

                    candidates.push(DeadlineCandidate {
                        deadline_id: Uuid::new_v4().to_string(),
                        commitment_id,
                        target_date,
                        confidence: 0.9,
                        evidence_ids: vec![event.id.clone()],
                        source_type: SourceType::RepositoryDerived,
                        created_at: Utc::now(),
                        updated_at: Utc::now(),
                    });
                }
            }
        }

        // 3. Inferred Temporal Acceleration
        // If a commitment has >= 3 sessions in under 24 hours, infer near-term deadline
        for commitment in commitments {
            let session_ids = &commitment.originating_sessions;
            if session_ids.len() >= 3 {
                let mut session_times = Vec::new();
                for s_id in session_ids {
                    if let Some(sess) = sessions.sessions().get(s_id) {
                        session_times.push(sess.start_timestamp);
                    }
                }
                
                if session_times.len() >= 3 {
                    session_times.sort();
                    let earliest = session_times.first().unwrap();
                    let latest = session_times.last().unwrap();
                    let span = (*latest - *earliest).num_seconds();

                    if span < 86400 { // under 24 hours
                        let target_date = commitment.last_activity_at + chrono::Duration::days(2);
                        candidates.push(DeadlineCandidate {
                            deadline_id: Uuid::new_v4().to_string(),
                            commitment_id: commitment.commitment_id.clone(),
                            target_date,
                            confidence: 0.8,
                            evidence_ids: commitment.evidence_ids.clone(),
                            source_type: SourceType::Inferred,
                            created_at: Utc::now(),
                            updated_at: Utc::now(),
                        });
                    }
                }
            }
        }

        candidates
    }

    /// Converts a DeadlineCandidate structure into a COM-compatible Event.
    pub fn to_event(candidate: &DeadlineCandidate) -> ChronosEvent {
        ChronosEvent::new(
            "DeadlineCandidateResolved",
            "DeadlineEngine",
            serde_json::to_value(candidate).unwrap_or(serde_json::Value::Null),
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
    fn test_explicit_deadline_discovery() {
        let mut resolver = EntityResolver::new();
        let disc = ChronosEvent::new("GitRepositoryDiscovered", "Git", json!({ "repository_path": "/workspace" }));
        resolver.process_event(&disc).unwrap();

        // Commit with explicit due date
        let commit = ChronosEvent::new(
            "GitCommitCreated",
            "GitAdapter",
            json!({
                "repository_path": "/workspace",
                "source_payload": { "message": "implement feature due 2025-12-01" }
            }),
        );
        resolver.process_event(&commit).unwrap();

        let graph = resolver.graph().clone();
        let mut session_engine = chronos_memory_sessions::SessionEngine::new(10, graph.clone());
        session_engine.process_event(&disc).unwrap();
        session_engine.process_event(&commit).unwrap();

        let state = StateProjector::project(&graph, session_engine.projection());
        
        let commitments = chronos_reasoning_commitments::CommitmentEngine::resolve_commitments(&state, &graph, session_engine.projection());
        
        let events = vec![disc, commit];
        let deadlines = DeadlineEngine::discover_deadlines(&state, &commitments, session_engine.projection(), &graph, &events);

        assert!(!deadlines.is_empty());
        let explicit = deadlines.iter().find(|d| d.source_type == SourceType::Explicit).unwrap();
        assert_eq!(explicit.target_date.format("%Y-%m-%d").to_string(), "2025-12-01");
    }

    #[test]
    fn test_repository_derived_deadline() {
        let mut resolver = EntityResolver::new();
        let disc = ChronosEvent::new("GitRepositoryDiscovered", "Git", json!({ "repository_path": "/workspace" }));
        resolver.process_event(&disc).unwrap();

        // Git Tag Created
        let tag = ChronosEvent::new(
            "GitTagCreated",
            "GitAdapter",
            json!({
                "repository_path": "/workspace",
                "source_payload": { "tag_name": "v1.0" }
            }),
        );
        resolver.process_event(&tag).unwrap();

        let graph = resolver.graph().clone();
        let mut session_engine = chronos_memory_sessions::SessionEngine::new(10, graph.clone());
        session_engine.process_event(&disc).unwrap();
        session_engine.process_event(&tag).unwrap();

        let state = StateProjector::project(&graph, session_engine.projection());
        
        let commitments = chronos_reasoning_commitments::CommitmentEngine::resolve_commitments(&state, &graph, session_engine.projection());
        
        let events = vec![disc, tag];
        let deadlines = DeadlineEngine::discover_deadlines(&state, &commitments, session_engine.projection(), &graph, &events);

        assert!(!deadlines.is_empty());
        let repo_derived = deadlines.iter().find(|d| d.source_type == SourceType::RepositoryDerived).unwrap();
        assert_eq!(repo_derived.confidence, 0.9);
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

        let events = vec![disc];
        let d1 = DeadlineEngine::discover_deadlines(&state, &commitments, session_engine.projection(), &graph, &events);
        let d2 = DeadlineEngine::discover_deadlines(&state, &commitments, session_engine.projection(), &graph, &events);

        assert_eq!(d1.len(), d2.len());
    }
}
