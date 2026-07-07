//! # Execution Orchestration Layer (EOL)
//!
//! EOL converts deterministic DecisionNodes into validated, replay-safe, event-sourced
//! real-world action executions, closing the loop between cognition and external
//! system interaction while preserving full auditability, determinism, and causal traceability.

use chrono::{DateTime, Utc};
use chronos_core::ChronosEvent;
use chronos_reasoning_decision::DecisionNode;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ExecutionActionType {
    ToolCall,
    APIRequest,
    InternalTask,
    DeferredExecution,
    NoOp,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ExecutionStatus {
    Pending,
    Running,
    Succeeded,
    Failed,
    Deferred,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OutcomeType {
    Success,
    PartialSuccess,
    Failure,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExecutionPlan {
    pub execution_plan_id: String,
    pub linked_decision_id: String,
    pub action_type: ExecutionActionType,
    pub execution_steps: Vec<String>,
    pub resource_requirements: HashMap<String, f64>,
    pub preconditions: Vec<String>,
    pub safety_constraints: Vec<String>,
    pub expected_side_effects: Vec<String>,
    pub provenance_chain: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExecutionNode {
    pub execution_id: String,
    pub linked_execution_plan_id: String,
    pub status: ExecutionStatus,
    pub start_timestamp: DateTime<Utc>,
    pub end_timestamp: Option<DateTime<Utc>>,
    pub result_payload: Option<serde_json::Value>,
    pub error_payload: Option<serde_json::Value>,
    pub retry_count: u32,
    pub determinism_signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExecutionOutcome {
    pub outcome_id: String,
    pub linked_execution_id: String,
    pub outcome_type: OutcomeType,
    pub observed_state_change: String,
    pub external_response_data: serde_json::Value,
    pub side_effect_log: Vec<String>,
    pub validation_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExecutionPlanCreatedPayload {
    pub plan: ExecutionPlan,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExecutionStartedPayload {
    pub execution_id: String,
    pub plan_id: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExecutionCompletedPayload {
    pub execution_id: String,
    pub result: serde_json::Value,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExecutionFailedPayload {
    pub execution_id: String,
    pub error: serde_json::Value,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExecutionOutcomeRecordedPayload {
    pub outcome: ExecutionOutcome,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ExecutionGraph {
    pub plans: HashMap<String, ExecutionPlan>,
    pub executions: HashMap<String, ExecutionNode>,
    pub outcomes: HashMap<String, ExecutionOutcome>,
}

impl ExecutionGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn apply_event(&mut self, event: &ChronosEvent) {
        match event.event_type.as_str() {
            "ExecutionPlanCreated" => {
                if let Ok(payload) = serde_json::from_value::<ExecutionPlanCreatedPayload>(event.payload.clone()) {
                    self.plans.insert(payload.plan.execution_plan_id.clone(), payload.plan);
                }
            }
            "ExecutionStarted" => {
                if let Ok(payload) = serde_json::from_value::<ExecutionStartedPayload>(event.payload.clone()) {
                    let node = ExecutionNode {
                        execution_id: payload.execution_id.clone(),
                        linked_execution_plan_id: payload.plan_id,
                        status: ExecutionStatus::Running,
                        start_timestamp: payload.timestamp,
                        end_timestamp: None,
                        result_payload: None,
                        error_payload: None,
                        retry_count: 0,
                        determinism_signature: format!("sig-{}", payload.execution_id),
                    };
                    self.executions.insert(payload.execution_id, node);
                }
            }
            "ExecutionCompleted" => {
                if let Ok(payload) = serde_json::from_value::<ExecutionCompletedPayload>(event.payload.clone()) {
                    if let Some(node) = self.executions.get_mut(&payload.execution_id) {
                        node.status = ExecutionStatus::Succeeded;
                        node.end_timestamp = Some(payload.timestamp);
                        node.result_payload = Some(payload.result);
                    }
                }
            }
            "ExecutionFailed" => {
                if let Ok(payload) = serde_json::from_value::<ExecutionFailedPayload>(event.payload.clone()) {
                    if let Some(node) = self.executions.get_mut(&payload.execution_id) {
                        node.status = ExecutionStatus::Failed;
                        node.end_timestamp = Some(payload.timestamp);
                        node.error_payload = Some(payload.error);
                    }
                }
            }
            "ExecutionOutcomeRecorded" => {
                if let Ok(payload) = serde_json::from_value::<ExecutionOutcomeRecordedPayload>(event.payload.clone()) {
                    self.outcomes.insert(payload.outcome.outcome_id.clone(), payload.outcome);
                }
            }
            _ => {}
        }
    }
}

pub trait ExternalExecutor: Send + Sync {
    fn execute(&self, plan: &ExecutionPlan) -> Result<serde_json::Value, serde_json::Value>;
}

pub struct DefaultMockExecutor;

impl ExternalExecutor for DefaultMockExecutor {
    fn execute(&self, _plan: &ExecutionPlan) -> Result<serde_json::Value, serde_json::Value> {
        Ok(serde_json::json!({ "status": "Task completed successfully" }))
    }
}

pub struct ExecutionOrchestrator;

impl ExecutionOrchestrator {
    /// Translates a DecisionNode into ExecutionPlan, processes execution steps, and logs results as events.
    pub fn process_decision(decision: &DecisionNode, executor: &dyn ExternalExecutor) -> Vec<ChronosEvent> {
        let mut events = Vec::new();

        // 1. Translation stage
        let plan_id = format!("plan-{}", decision.decision_id);
        let plan = ExecutionPlan {
            execution_plan_id: plan_id.clone(),
            linked_decision_id: decision.decision_id.clone(),
            action_type: ExecutionActionType::InternalTask,
            execution_steps: vec!["Validate".to_string(), "Dispatch".to_string(), "Verify".to_string()],
            resource_requirements: HashMap::new(),
            preconditions: vec!["SystemReady".to_string()],
            safety_constraints: vec!["NoDestructiveWrite".to_string()],
            expected_side_effects: vec!["InternalLogsCreated".to_string()],
            provenance_chain: vec![decision.selected_candidate_id.clone()],
        };

        events.push(ChronosEvent::new(
            "ExecutionPlanCreated",
            "ExecutionOrchestrator",
            serde_json::to_value(ExecutionPlanCreatedPayload { plan: plan.clone() }).unwrap(),
        ));

        // 2. Dispatch / Run simulation stage
        let exec_id = format!("exec-{}", decision.decision_id);
        events.push(ChronosEvent::new(
            "ExecutionStarted",
            "ExecutionOrchestrator",
            serde_json::to_value(ExecutionStartedPayload {
                execution_id: exec_id.clone(),
                plan_id: plan_id.clone(),
                timestamp: Utc::now(),
            }).unwrap(),
        ));

        // 3. Actual Execution via ExternalExecutor
        let execution_result = executor.execute(&plan);

        match execution_result {
            Ok(result_payload) => {
                events.push(ChronosEvent::new(
                    "ExecutionCompleted",
                    "ExecutionOrchestrator",
                    serde_json::to_value(ExecutionCompletedPayload {
                        execution_id: exec_id.clone(),
                        result: result_payload.clone(),
                        timestamp: Utc::now(),
                    }).unwrap(),
                ));

                // Capture Outcome
                let outcome = ExecutionOutcome {
                    outcome_id: format!("outcome-{}", exec_id),
                    linked_execution_id: exec_id.clone(),
                    outcome_type: OutcomeType::Success,
                    observed_state_change: "SystemStateUpdated".to_string(),
                    external_response_data: result_payload,
                    side_effect_log: vec!["Logged state modification".to_string()],
                    validation_hash: format!("val-hash-{}", exec_id),
                };

                events.push(ChronosEvent::new(
                    "ExecutionOutcomeRecorded",
                    "ExecutionOrchestrator",
                    serde_json::to_value(ExecutionOutcomeRecordedPayload { outcome }).unwrap(),
                ));
            }
            Err(error_payload) => {
                events.push(ChronosEvent::new(
                    "ExecutionFailed",
                    "ExecutionOrchestrator",
                    serde_json::to_value(ExecutionFailedPayload {
                        execution_id: exec_id.clone(),
                        error: error_payload.clone(),
                        timestamp: Utc::now(),
                    }).unwrap(),
                ));

                // Capture Failed Outcome
                let outcome = ExecutionOutcome {
                    outcome_id: format!("outcome-{}", exec_id),
                    linked_execution_id: exec_id.clone(),
                    outcome_type: OutcomeType::Failure,
                    observed_state_change: "None".to_string(),
                    external_response_data: error_payload,
                    side_effect_log: vec!["Execution failed".to_string()],
                    validation_hash: format!("val-hash-{}", exec_id),
                };

                events.push(ChronosEvent::new(
                    "ExecutionOutcomeRecorded",
                    "ExecutionOrchestrator",
                    serde_json::to_value(ExecutionOutcomeRecordedPayload { outcome }).unwrap(),
                ));
            }
        }

        events
    }

    /// Rebuilds ExecutionGraph deterministically from history.
    pub fn rebuild_execution_graph(events: &[ChronosEvent]) -> ExecutionGraph {
        let mut graph = ExecutionGraph::new();
        for event in events {
            graph.apply_event(event);
        }
        graph
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chronos_reasoning_decision::DecisionNode;
    use serde_json::json;

    #[test]
    fn test_decision_to_execution_replay() {
        let decision = DecisionNode {
            decision_id: "test-dec-1".to_string(),
            selected_candidate_id: "cand-1".to_string(),
            priority_rank: 1,
            selection_reason_chain: vec!["Highest score".to_string()],
            competing_rejected_candidates: vec![],
            resource_allocation_estimate: 1.0,
            expected_outcome_type: "Success".to_string(),
            timestamp: Utc::now(),
            stability_signature: "stable-1".to_string(),
        };

        let events = ExecutionOrchestrator::process_decision(&decision, &DefaultMockExecutor);

        // Check plan creation, running state, completion and outcomes are produced
        assert!(events.iter().any(|e| e.event_type == "ExecutionPlanCreated"));
        assert!(events.iter().any(|e| e.event_type == "ExecutionStarted"));
        assert!(events.iter().any(|e| e.event_type == "ExecutionCompleted"));
        assert!(events.iter().any(|e| e.event_type == "ExecutionOutcomeRecorded"));

        let graph = ExecutionOrchestrator::rebuild_execution_graph(&events);
        assert!(graph.plans.contains_key("plan-test-dec-1"));
        assert_eq!(graph.executions.get("exec-test-dec-1").unwrap().status, ExecutionStatus::Succeeded);

        // Replay determinism check
        let replayed = ExecutionOrchestrator::rebuild_execution_graph(&events);
        assert_eq!(graph, replayed);
    }
}
