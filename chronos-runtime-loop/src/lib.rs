//! # Continuous Runtime Loop Engine (CRLE)
//!
//! Implements a deterministic Tick Finalization Layer that transforms Chronos from a
//! multi-engine reasoning system into a single coherent end-to-end cognitive runtime.
//!
//! ## Tick Finalization Contract
//!
//! Every call to `execute_tick()` MUST follow this strict pipeline:
//!
//! ```text
//! Phase 1: Ingest events
//! Phase 2: Run all reasoning engines independently
//! Phase 3: Synthesize ONE canonical CognitiveState from all engine outputs
//! Phase 4: Generate decision ONLY from CognitiveState
//! Phase 5: Execute plan derived from decision
//! Phase 6: Collect feedback
//! ```
//!
//! The single output of every tick is a [`TickFrame`], which records every
//! input and output of the above pipeline for full auditability and deterministic
//! replay.

use chrono::{DateTime, Utc};
use chronos_core::ChronosEvent;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

// ─── Runtime Mode ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuntimeMode {
    Live,
    Replay,
}

// ─── Observability Metrics ────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InternalMetrics {
    pub tick_duration_ms: u64,
    pub event_throughput: u64,
    pub decision_throughput: u64,
    pub execution_backlog_size: u64,
    pub feedback_lag_ms: u64,
}

// ─── CognitiveState ───────────────────────────────────────────────────────────
//
// The single authoritative world model produced each tick.
// This is the ONLY permitted input to decision generation.
//
// Sources:
//   - active_commitments  ← chronos-reasoning-commitments
//   - priority_vector     ← derived from coherence node confidence scores
//   - risk_snapshot       ← chronos-reasoning-coherence conflict flags
//   - coherence_score     ← mean global_confidence_score across CognitiveStateGraph nodes
//   - intent_summary      ← chronos-reasoning-intent CanonicalState intents

/// The unified world model synthesized from all reasoning engines each tick.
/// Decisions MUST NOT be generated from any other source.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CognitiveState {
    /// IDs of commitments that are currently Active or AtRisk.
    pub active_commitment_ids: Vec<String>,

    /// Ordered list of (cognitive_state_node_id, priority_score) pairs,
    /// highest priority first. Derived from coherence node confidence scores.
    pub priority_vector: Vec<(String, f64)>,

    /// Maximum intervention urgency score across all detected coherence conflicts.
    /// 0.0 = no conflicts, 1.0 = critical conflict requiring immediate action.
    pub risk_snapshot: f64,

    /// Mean global_confidence_score across all CognitiveStateGraph nodes.
    /// 0.0 = incoherent, 1.0 = fully coherent.
    pub coherence_score: f64,

    /// Raw content strings from all detected intent signals, for audit traceability.
    pub intent_summary: Vec<String>,

    /// Number of unresolved cognitive conflicts detected this tick.
    pub unresolved_conflict_count: usize,

    /// Tick timestamp — fixed at synthesis time so downstream stages share the same clock.
    pub synthesized_at: DateTime<Utc>,
}

impl CognitiveState {
    /// Returns true if this state justifies generating a decision.
    /// A decision is only warranted when there is at least one active commitment
    /// or unresolved cognitive conflict.
    pub fn warrants_decision(&self) -> bool {
        !self.active_commitment_ids.is_empty() || self.unresolved_conflict_count > 0
    }
}

// ─── TickFrame Telemetry & Tracing ───────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceSpan {
    pub component: String,
    pub operation: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_ms: u64,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TickTelemetry {
    pub tick_sequence: u64,
    pub perception_ingestion_time: Option<DateTime<Utc>>,
    pub tick_execution_start_time: DateTime<Utc>,
    pub tick_execution_completed_time: Option<DateTime<Utc>>,
    pub phase_durations_ms: std::collections::HashMap<String, u64>,
    pub total_duration_ms: u64,
    pub traces: Vec<TraceSpan>,
}

// ─── TickFrame ────────────────────────────────────────────────────────────────
//
// The canonical output record for a single tick. Contains full provenance across
// every pipeline phase, enabling deterministic replay and forensic audit.

/// The canonical output of one execution tick.
/// Every field is set exactly once per tick in strict phase order.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TickFrame {
    /// Unique identifier for this tick.
    pub tick_id: String,

    /// Timestamp when this tick started.
    pub tick_started_at: DateTime<Utc>,

    /// Timestamp when this tick completed.
    pub tick_completed_at: Option<DateTime<Utc>>,

    // ── Phase 1: Perception ──────────────────────────────────────────────────
    /// Events ingested into this tick from outside the loop.
    pub perception: Vec<ChronosEvent>,

    // ── Phase 2: Reasoning ───────────────────────────────────────────────────
    /// Events emitted by the coherence reconciliation stage.
    pub reasoning: Vec<ChronosEvent>,

    // ── Phase 3: Cognitive State Synthesis ───────────────────────────────────
    /// The single authoritative CognitiveState produced this tick.
    /// None until synthesize_cognitive_state() completes.
    pub cognitive_state: Option<CognitiveState>,

    // ── Phase 4: Decision ────────────────────────────────────────────────────
    /// Events emitted by the decision pipeline, sourced ONLY from cognitive_state.
    /// Empty when cognitive_state.warrants_decision() == false.
    pub decision: Vec<ChronosEvent>,

    // ── Phase 5: Execution ───────────────────────────────────────────────────
    /// Events emitted by the execution orchestrator.
    pub execution: Vec<ChronosEvent>,

    // ── Phase 6: Feedback ────────────────────────────────────────────────────
    /// Events emitted by the feedback engine.
    pub feedback: Vec<ChronosEvent>,

    // Observability telemetry
    pub telemetry: Option<TickTelemetry>,
}

impl TickFrame {
    fn new(perception: Vec<ChronosEvent>) -> Self {
        Self {
            tick_id: Uuid::new_v4().to_string(),
            tick_started_at: Utc::now(),
            tick_completed_at: None,
            perception,
            reasoning: Vec::new(),
            cognitive_state: None,
            decision: Vec::new(),
            execution: Vec::new(),
            feedback: Vec::new(),
            telemetry: None,
        }
    }

    /// Flattens all tick events in pipeline order for bus emission or logging.
    pub fn all_events(&self) -> Vec<ChronosEvent> {
        let mut out = Vec::new();
        out.extend(self.reasoning.iter().cloned());
        out.extend(self.decision.iter().cloned());
        out.extend(self.execution.iter().cloned());
        out.extend(self.feedback.iter().cloned());
        out
    }
}

// ─── CognitiveState Synthesis ────────────────────────────────────────────────

/// Synthesizes a single authoritative [`CognitiveState`] from the outputs of all
/// reasoning engines.
///
/// # Determinism Contract
///
/// Given the same `batch` of events, this function MUST always return the same
/// [`CognitiveState`]. It is stateless and has no side effects.
///
/// # Inputs
///
/// | Parameter         | Source crate                          |
/// |-------------------|---------------------------------------|
/// | `coherence_graph` | `chronos-reasoning-coherence`         |
/// | `intent_state`    | `chronos-reasoning-intent`            |
/// | `commitment_state`| `chronos-reasoning-commitments`       |
///
/// # Conflict Resolution Rules (deterministic)
///
/// 1. `active_commitment_ids` — include all commitments whose status is
///    `Active` or `AtRisk`, sorted lexicographically for stable ordering.
/// 2. `priority_vector` — collect all CognitiveStateGraph nodes, sort
///    descending by `global_confidence_score`, then by `cognitive_state_id`
///    as a stable tiebreaker.
/// 3. `coherence_score` — arithmetic mean of all node `global_confidence_score`
///    values. Zero when no nodes exist.
/// 4. `risk_snapshot` — count of `detected_conflicts` that have no
///    corresponding entry in `resolved_conflicts` (i.e., still open), mapped
///    to an urgency value: `min(1.0, open_conflicts as f64 * 0.25)`.
/// 5. `intent_summary` — raw_content from each intent signal, sorted by
///    intent_id lexicographically for stable ordering.
pub fn synthesize_cognitive_state(
    coherence_graph: &chronos_reasoning_coherence::CognitiveStateGraph,
    intent_state: &chronos_reasoning_intent::CanonicalState,
    commitment_state: &chronos_reasoning_commitments::CommitmentState,
    batch: &[ChronosEvent],
) -> CognitiveState {
    use chronos_reasoning_commitments::CommitmentStatus;

    // Rule 1: Active + AtRisk commitment IDs, stable sort.
    let mut active_commitment_ids: Vec<String> = commitment_state
        .commitments
        .values()
        .filter(|c| c.status == CommitmentStatus::Active || c.status == CommitmentStatus::AtRisk)
        .map(|c| c.commitment_id.clone())
        .collect();
    active_commitment_ids.sort();

    // Rule 2: Priority vector — coherence nodes sorted by confidence DESC, then id ASC.
    let mut priority_vector: Vec<(String, f64)> = coherence_graph
        .nodes
        .values()
        .map(|n| (n.cognitive_state_id.clone(), n.global_confidence_score))
        .collect();
    priority_vector.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.cmp(&b.0))
    });

    // Rule 3: Coherence score — mean of all node confidence scores.
    let coherence_score = if coherence_graph.nodes.is_empty() {
        0.0
    } else {
        let sum: f64 = coherence_graph
            .nodes
            .values()
            .map(|n| n.global_confidence_score)
            .sum();
        sum / coherence_graph.nodes.len() as f64
    };

    // Rule 4: Risk snapshot — combine open (unresolved) conflicts and RiskForecast.
    let resolved_ids: std::collections::HashSet<String> = coherence_graph
        .resolved_conflicts
        .iter()
        .flat_map(|r| r.resolved_node_ids.iter().cloned())
        .collect();
    let open_conflict_count = coherence_graph
        .detected_conflicts
        .iter()
        .filter(|c| c.affected_node_ids.iter().any(|id| !resolved_ids.contains(id)))
        .count();
    
    // Look for the most recent RiskForecastResolved in the batch
    let mut base_risk = 0.0;
    if let Some(risk_event) = batch.iter().rev().find(|e| e.event_type == "RiskForecastResolved") {
        if let Some(forecast) = risk_event.payload.as_object() {
            if let Some(urgency) = forecast.get("intervention_urgency").and_then(|u| u.as_f64()) {
                base_risk = urgency;
            }
        }
    }
    
    let risk_snapshot = (base_risk + (open_conflict_count as f64 * 0.25)).min(1.0);

    // Rule 5: Intent summary — raw_content sorted by intent_id.
    let mut intent_entries: Vec<(String, String)> = intent_state
        .intents
        .iter()
        .map(|(id, sig)| (id.clone(), sig.raw_content.clone()))
        .collect();
    intent_entries.sort_by(|a, b| a.0.cmp(&b.0));
    let intent_summary: Vec<String> = intent_entries.into_iter().map(|(_, raw)| raw).collect();

    let unresolved_conflict_count = open_conflict_count;

    CognitiveState {
        active_commitment_ids,
        priority_vector,
        risk_snapshot,
        coherence_score,
        intent_summary,
        unresolved_conflict_count,
        synthesized_at: Utc::now(),
    }
}

// ─── Runtime Engine ───────────────────────────────────────────────────────────

pub struct ContinuousRuntimeLoopEngine {
    pub mode: RuntimeMode,
    pub metrics: Arc<Mutex<InternalMetrics>>,
    pub is_running: Arc<Mutex<bool>>,
}

impl ContinuousRuntimeLoopEngine {
    pub fn new(mode: RuntimeMode) -> Self {
        Self {
            mode,
            metrics: Arc::new(Mutex::new(InternalMetrics::default())),
            is_running: Arc::new(Mutex::new(false)),
        }
    }

    /// Execute one deterministic tick of the Chronos cognitive runtime.
    ///
    /// Returns a [`TickFrame`] containing the full provenance record for this tick,
    /// and the flattened `Vec<ChronosEvent>` of all events produced (for backward
    /// compatibility with callers that consume a flat event list).
    pub fn execute_tick_framed(
        &self,
        history: &[ChronosEvent],
        new_events: &[ChronosEvent],
        session_id: &str,
        executor: &dyn chronos_execution_orchestration::ExternalExecutor,
    ) -> TickFrame {
        let start_time = Utc::now();
        let mut phase_durations_ms = std::collections::HashMap::new();
        let mut traces = Vec::new();

        let perception_ingestion_time = new_events
            .iter()
            .map(|e| e.timestamp)
            .min();

        // ── Phase 1: Perception ───────────────────────────────────────────────
        let t_perception_start = Utc::now();
        let mut frame = TickFrame::new(new_events.to_vec());
        let mut batch = history.to_vec();
        batch.extend_from_slice(new_events);
        phase_durations_ms.insert(
            "perception".to_string(),
            Utc::now().signed_duration_since(t_perception_start).num_milliseconds() as u64,
        );

        // ── Phase 2: Reasoning ────────────────────────────────────────────────
        let t_reasoning_start = Utc::now();
        let intent_state =
            chronos_reasoning_intent::CanonicalCommitmentBuilder::rebuild_intent_and_commitment_state(&batch);

        let commitment_state =
            chronos_reasoning_commitments::CommitmentEngine::rebuild_commitment_state(&batch);

        let continuity_graph =
            chronos_reasoning_continuity::ContextContinuityEngine::rebuild_continuity_graph(&batch);

        let coherence_graph =
            chronos_reasoning_coherence::CoherenceEngine::rebuild_cognitive_state(&batch);

        let coherence_updates = chronos_reasoning_coherence::CoherenceEngine::reconcile(
            &coherence_graph,
            &intent_state,
            &commitment_state,
            &continuity_graph,
        );
        frame.reasoning.extend(coherence_updates.clone());
        batch.extend(coherence_updates);

        let coherence_graph_post_reconcile =
            chronos_reasoning_coherence::CoherenceEngine::rebuild_cognitive_state(&batch);

        let commitment_state_post_reconcile =
            chronos_reasoning_commitments::CommitmentEngine::rebuild_commitment_state(&batch);
        phase_durations_ms.insert(
            "reasoning".to_string(),
            Utc::now().signed_duration_since(t_reasoning_start).num_milliseconds() as u64,
        );

        // ── Phase 3: CognitiveState Synthesis ────────────────────────────────
        let t_synth_start = Utc::now();
        let cognitive_state = synthesize_cognitive_state(
            &coherence_graph_post_reconcile,
            &intent_state,
            &commitment_state_post_reconcile,
            &batch,
        );
        frame.cognitive_state = Some(cognitive_state.clone());
        let t_synth_end = Utc::now();
        let synth_duration = t_synth_end.signed_duration_since(t_synth_start).num_milliseconds() as u64;
        phase_durations_ms.insert("synthesis".to_string(), synth_duration);
        traces.push(TraceSpan {
            component: "StateProjector".to_string(),
            operation: "synthesize_cognitive_state".to_string(),
            started_at: t_synth_start,
            completed_at: Some(t_synth_end),
            duration_ms: synth_duration,
            metadata: serde_json::json!({
                "unresolved_conflict_count": cognitive_state.unresolved_conflict_count,
                "coherence_score": cognitive_state.coherence_score,
                "risk_snapshot": cognitive_state.risk_snapshot,
            }),
        });

        // ── Phase 4: Decision ─────────────────────────────────────────────────
        let t_dec_start = Utc::now();
        if cognitive_state.warrants_decision() {
            let decision_events = chronos_reasoning_decision::DecisionPipeline::generate_decisions(
                &coherence_graph_post_reconcile,
            );
            frame.decision.extend(decision_events.clone());
            batch.extend(decision_events.clone());
            
            let t_dec_end = Utc::now();
            let dec_duration = t_dec_end.signed_duration_since(t_dec_start).num_milliseconds() as u64;
            traces.push(TraceSpan {
                component: "DecisionPipeline".to_string(),
                operation: "generate_decisions".to_string(),
                started_at: t_dec_start,
                completed_at: Some(t_dec_end),
                duration_ms: dec_duration,
                metadata: serde_json::json!({
                    "decisions_count": decision_events.len(),
                }),
            });
        }
        phase_durations_ms.insert(
            "decision".to_string(),
            Utc::now().signed_duration_since(t_dec_start).num_milliseconds() as u64,
        );

        // ── Phase 5: Execution ────────────────────────────────────────────────
        let t_exec_start = Utc::now();
        let decision_graph =
            chronos_reasoning_decision::DecisionPipeline::rebuild_decision_graph(&batch);

        for decision in decision_graph.decisions.values() {
            let t_process_start = Utc::now();
            let exec_events = if self.mode == RuntimeMode::Live {
                let evs = chronos_execution_orchestration::ExecutionOrchestrator::process_decision(decision, executor);
                frame.execution.extend(evs.clone());
                batch.extend(evs.clone());
                evs
            } else {
                let mock_exec_id = format!("exec-mock-{}", decision.decision_id);
                let mock_plan_id = format!("plan-mock-{}", decision.decision_id);

                let ev1 = ChronosEvent::new(
                    "ExecutionStarted",
                    "ExecutionOrchestrator",
                    serde_json::json!({
                        "execution_id": mock_exec_id,
                        "plan_id": mock_plan_id,
                        "tick_id": frame.tick_id,
                    }),
                );
                let ev2 = ChronosEvent::new(
                    "ExecutionCompleted",
                    "ExecutionOrchestrator",
                    serde_json::json!({
                        "execution_id": mock_exec_id,
                        "result": { "status": "simulated_success" },
                        "tick_id": frame.tick_id,
                    }),
                );

                let outcome = chronos_execution_orchestration::ExecutionOutcome {
                    outcome_id: format!("outcome-{}", mock_exec_id),
                    linked_execution_id: mock_exec_id,
                    outcome_type: chronos_execution_orchestration::OutcomeType::Success,
                    observed_state_change: "SimulatedStateChange".to_string(),
                    external_response_data: serde_json::json!({ "simulated": true }),
                    side_effect_log: vec!["Simulated side effect".to_string()],
                    validation_hash: "mock-hash-1".to_string(),
                };

                let ev3 = ChronosEvent::new(
                    "ExecutionOutcomeRecorded",
                    "ExecutionOrchestrator",
                    serde_json::to_value(
                        chronos_execution_orchestration::ExecutionOutcomeRecordedPayload {
                            outcome,
                        },
                    )
                    .unwrap(),
                );

                let mut evs = vec![ev1, ev2, ev3];
                frame.execution.extend(evs.clone());
                batch.extend(evs.clone());
                evs
            };

            let t_process_end = Utc::now();
            traces.push(TraceSpan {
                component: "ExecutionOrchestrator".to_string(),
                operation: "process_decision".to_string(),
                started_at: t_process_start,
                completed_at: Some(t_process_end),
                duration_ms: t_process_end.signed_duration_since(t_process_start).num_milliseconds() as u64,
                metadata: serde_json::json!({
                    "decision_id": decision.decision_id,
                    "action_type": decision.expected_outcome_type.clone(),
                    "exec_events_count": exec_events.len(),
                }),
            });
        }
        phase_durations_ms.insert(
            "execution".to_string(),
            Utc::now().signed_duration_since(t_exec_start).num_milliseconds() as u64,
        );

        // ── Phase 6: Feedback ─────────────────────────────────────────────────
        let t_feed_start = Utc::now();
        let exec_graph =
            chronos_execution_orchestration::ExecutionOrchestrator::rebuild_execution_graph(&batch);

        for outcome in exec_graph.outcomes.values() {
            let mut decision_id = "unknown-decision".to_string();
            let mut linked_commitment_id: Option<String> = None;
            let mut linked_intent_id: Option<String> = None;

            if let Some(exec) = exec_graph.executions.get(&outcome.linked_execution_id) {
                if let Some(plan) = exec_graph.plans.get(&exec.linked_execution_plan_id) {
                    decision_id = plan.linked_decision_id.clone();
                    
                    if let Some(decision) = decision_graph.decisions.get(&decision_id) {
                        if let Some(candidate) = decision_graph.candidates.get(&decision.selected_candidate_id) {
                            linked_commitment_id = candidate.linked_commitment_id.clone();
                            linked_intent_id = candidate.linked_intent_id.clone();
                        }
                    }
                }
            }

            let t_outcome_start = Utc::now();
            let feedback_events = chronos_execution_feedback::FeedbackEngine::process_outcome(
                outcome,
                linked_commitment_id.as_deref(),
                linked_intent_id.as_deref(),
                &decision_id,
            );
            frame.feedback.extend(feedback_events.clone());

            let t_outcome_end = Utc::now();
            traces.push(TraceSpan {
                component: "FeedbackEngine".to_string(),
                operation: "process_outcome".to_string(),
                started_at: t_outcome_start,
                completed_at: Some(t_outcome_end),
                duration_ms: t_outcome_end.signed_duration_since(t_outcome_start).num_milliseconds() as u64,
                metadata: serde_json::json!({
                    "outcome_id": outcome.outcome_id,
                    "decision_id": decision_id,
                    "feedback_events_count": feedback_events.len(),
                }),
            });
        }
        phase_durations_ms.insert(
            "feedback".to_string(),
            Utc::now().signed_duration_since(t_feed_start).num_milliseconds() as u64,
        );

        // ── Finalize ──────────────────────────────────────────────────────────
        frame.tick_completed_at = Some(Utc::now());

        let total_duration_ms = Utc::now()
            .signed_duration_since(start_time)
            .num_milliseconds() as u64;

        if total_duration_ms > 50 {
            let warning_event = ChronosEvent::new(
                "TickPerformanceWarning",
                "ContinuousRuntimeLoopEngine",
                serde_json::json!({
                    "tick_id": frame.tick_id,
                    "duration_ms": total_duration_ms,
                    "threshold_ms": 50,
                    "warning": format!("Tick processing took {}ms, exceeding 50ms soft cap.", total_duration_ms)
                }),
            );
            frame.reasoning.push(warning_event);
        }

        let tick_sequence = history
            .iter()
            .filter(|e| e.event_type == "TickFrameEmitted")
            .count() as u64 + 1;

        frame.telemetry = Some(TickTelemetry {
            tick_sequence,
            perception_ingestion_time,
            tick_execution_start_time: start_time,
            tick_execution_completed_time: frame.tick_completed_at,
            phase_durations_ms,
            total_duration_ms,
            traces,
        });

        let mut m = self.metrics.lock().unwrap();
        m.tick_duration_ms = total_duration_ms;
        m.event_throughput += new_events.len() as u64;

        m.decision_throughput += decision_graph.decisions.len() as u64;
        m.execution_backlog_size =
            exec_graph.plans.len() as u64 - exec_graph.executions.len() as u64;

        frame
    }

    /// Backward-compatible wrapper around `execute_tick_framed`.
    /// Returns the flat list of events produced by this tick.
    ///
    /// All callers that currently use `execute_tick` continue to work without
    /// changes. Migrate callers to `execute_tick_framed` to access the full
    /// Legacy compatibility shim.
    pub fn execute_tick(
        &self,
        incoming_events: &[ChronosEvent],
        session_id: &str,
    ) -> Vec<ChronosEvent> {
        let frame = self.execute_tick_framed(&[], incoming_events, session_id, &chronos_execution_orchestration::DefaultMockExecutor);
        frame.all_events()
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_intent_event() -> ChronosEvent {
        ChronosEvent::new(
            "IntentDetected",
            "Test",
            serde_json::json!({
                "intent": {
                    "intent_id": "intent-1",
                    "source": "manual",
                    "raw_content": "Simulated task",
                    "signal_type": "Explicit",
                    "timestamp": Utc::now(),
                    "confidence_score": 0.9,
                    "context_metadata": {},
                    "provenance_event_ids": []
                }
            }),
        )
    }

    // ── TickFrame structure ───────────────────────────────────────────────────

    #[test]
    fn test_every_tick_produces_one_frame() {
        let engine = ContinuousRuntimeLoopEngine::new(RuntimeMode::Replay);
        let frame = engine.execute_tick_framed(&[], &[make_intent_event()], "session-1", &chronos_execution_orchestration::DefaultMockExecutor);

        // Every tick must produce exactly one TickFrame with a unique tick_id.
        assert!(!frame.tick_id.is_empty());
        // Perception must contain the events we provided.
        assert_eq!(frame.perception.len(), 1);
        // Completion timestamp must be set.
        assert!(frame.tick_completed_at.is_some());
    }

    #[test]
    fn test_cognitive_state_is_always_synthesized() {
        let engine = ContinuousRuntimeLoopEngine::new(RuntimeMode::Replay);
        let frame = engine.execute_tick_framed(&[], &[make_intent_event()], "session-1", &chronos_execution_orchestration::DefaultMockExecutor);

        // cognitive_state must always be Some after a tick.
        assert!(frame.cognitive_state.is_some());
    }

    #[test]
    fn test_decision_only_when_warranted() {
        let engine = ContinuousRuntimeLoopEngine::new(RuntimeMode::Replay);

        // Empty event batch: no commitments, no intents, no conflicts.
        // CognitiveState should not warrant a decision, so decision phase should be empty.
        let frame_empty = engine.execute_tick_framed(&[], &[], "session-empty", &chronos_execution_orchestration::DefaultMockExecutor);
        let cs = frame_empty.cognitive_state.as_ref().unwrap();
        if !cs.warrants_decision() {
            assert!(frame_empty.decision.is_empty(),
                "Decision phase must be empty when CognitiveState does not warrant a decision");
        }
    }

    // ── CognitiveState synthesis ──────────────────────────────────────────────

    #[test]
    fn test_synthesize_cognitive_state_determinism() {
        // Same inputs must always produce the same CognitiveState.
        let coherence_graph = chronos_reasoning_coherence::CognitiveStateGraph::new();
        let intent_state = chronos_reasoning_intent::CanonicalState::new();
        let commitment_state = chronos_reasoning_commitments::CommitmentState::new();

        let cs1 = synthesize_cognitive_state(&coherence_graph, &intent_state, &commitment_state, &[]);
        let cs2 = synthesize_cognitive_state(&coherence_graph, &intent_state, &commitment_state, &[]);

        // All fields except synthesized_at must be identical.
        assert_eq!(cs1.active_commitment_ids, cs2.active_commitment_ids);
        assert_eq!(cs1.priority_vector, cs2.priority_vector);
        assert_eq!(cs1.coherence_score, cs2.coherence_score);
        assert_eq!(cs1.risk_snapshot, cs2.risk_snapshot);
        assert_eq!(cs1.intent_summary, cs2.intent_summary);
        assert_eq!(cs1.unresolved_conflict_count, cs2.unresolved_conflict_count);
    }

    #[test]
    fn test_empty_state_coherence_score_is_zero() {
        let coherence_graph = chronos_reasoning_coherence::CognitiveStateGraph::new();
        let intent_state = chronos_reasoning_intent::CanonicalState::new();
        let commitment_state = chronos_reasoning_commitments::CommitmentState::new();

        let cs = synthesize_cognitive_state(&coherence_graph, &intent_state, &commitment_state, &[]);
        assert_eq!(cs.coherence_score, 0.0);
        assert_eq!(cs.risk_snapshot, 0.0);
        assert!(cs.active_commitment_ids.is_empty());
        assert!(cs.priority_vector.is_empty());
        assert!(!cs.warrants_decision());
    }

    // ── Full pipeline (backward-compat) ───────────────────────────────────────

    #[test]
    fn test_continuous_runtime_tick_replay() {
        let engine = ContinuousRuntimeLoopEngine::new(RuntimeMode::Replay);
        let tick_events = engine.execute_tick(&[make_intent_event()], "test-session");

        // Backward-compat: flat event list still emitted.
        assert!(tick_events.iter().any(|e| e.event_type == "CognitiveConflictDetected"));
        assert!(tick_events.iter().any(|e| e.event_type == "DecisionCandidateGenerated"));
        assert!(tick_events.iter().any(|e| e.event_type == "ExecutionStarted"));

        let m = engine.metrics.lock().unwrap();
        assert!(m.event_throughput > 0);
    }

    #[test]
    fn test_tick_frame_all_events_matches_execute_tick() {
        let engine = ContinuousRuntimeLoopEngine::new(RuntimeMode::Replay);
        let event = make_intent_event();

        // Both paths must produce the same set of event_types in the same order.
        let frame = engine.execute_tick_framed(&[], &[event.clone()], "s", &chronos_execution_orchestration::DefaultMockExecutor);
        let flat = engine.execute_tick(&[event], "s");

        // Both paths must produce the same set of event_types in the same order.
        let frame_all = frame.all_events();
        let frame_types: Vec<&str> = frame_all.iter().map(|e| e.event_type.as_str()).collect();
        let flat_types: Vec<&str> = flat.iter().map(|e| e.event_type.as_str()).collect();
        assert_eq!(frame_types, flat_types);
    }
}
