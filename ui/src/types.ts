/**
 * @license
 * SPDX-License-Identifier: Apache-2.0
 */

export interface DeadLetterQueueItem {
  id: number;
  source_uri: string;
  payload_hash: string;
  worker_type: string;
  failure_reason: string;
  retry_count: number;
  failed_at: string;
}

export interface Project {
  id: number;
  project_name: string;
  status: 'ACTIVE' | 'ARCHIVED';
  created_at: string;
}

export interface ProjectState {
  project_id: number;
  current_summary: string;
  current_entry_point?: string;
  next_action?: string;
  confidence_score: number;
  updated_at: string;
}

export interface ContextNode {
  id: number;
  project_id: number | null;
  entity_key: string; // e.g. 'FILE:src/billing/stripe.ts' or 'DOMAIN:docs.stripe.com'
  entity_type: 'FILE' | 'URL' | 'DOCUMENT' | 'RESEARCH_TOPIC' | 'RESEARCH_SESSION';
  display_name: string;
  created_at: string;
}

export interface ContextEvent {
  id: number;
  node_id: number;
  event_type: 'CREATED' | 'OPENED' | 'EDITED' | 'REFERENCED' | 'TAB_FOCUS';
  interaction_duration: number; // Active focus seconds
  captured_at: string;
}

export interface BrowserSession {
  id: number;
  project_id: number | null;
  url: string;
  page_title: string;
  domain: string;
  visit_started_at: string;
  visit_ended_at?: string;
  active_seconds: number;
  created_at: string;
}

export interface SearchQuery {
  id: number;
  browser_session_id: number;
  query_text: string;
  created_at: string;
}

export interface Commitment {
  id: number;
  project_id: number;
  title: string;
  commitment_type: 'ASSIGNMENT' | 'DELIVERABLE' | 'MEETING' | 'OBLIGATION';
  deadline_date?: string; // ISO-8601 string
  confidence_score: number;
  source_node_id: number | null;
  status: 'OPEN' | 'COMPLETED' | 'ABANDONED';
  created_at: string;
  // Computed client-side fields
  health?: 'GREEN' | 'YELLOW' | 'RED';
  risk_score?: number;
  completion_chance?: number;
}

export interface ProjectDeadline {
  id: number;
  project_id: number;
  deadline_label: string;
  target_date: string; // ISO-8601 string
  importance_tier: 'LOW' | 'MEDIUM' | 'HIGH' | 'CRITICAL';
  created_at: string;
}

export interface ProjectAction {
  id: number;
  project_id: number;
  action_text: string;
  estimated_effort_hours: number;
  status: 'PENDING' | 'COMPLETED' | 'DEPRECATED';
  priority_score: number;
  created_at: string;
  why_now_diagnostics?: string[];
}

export interface ProjectCheckpoint {
  id: number;
  project_id: number;
  accomplished_text: string;
  blocked_text?: string;
  next_steps_text?: string;
  created_at: string;
}

export interface RecoveryPlan {
  id: number;
  commitment_id: number;
  plan_payload_json: string; // Serialized string containing schedule checklist array
  generated_at: string;
}

export interface AutonomousResearchBrief {
  id: number;
  project_id: number;
  brief_payload_json: string; // Serialized string containing list of matches, summaries, links
  generated_at: string;
}

export interface GraphEdge {
  source_node_id: number;
  target_node_id: number;
  edge_type: 'REFERENCES' | 'GENERATED_FROM' | 'SUPPORTS' | 'BLOCKS' | 'RELATED_TO' | 'INVESTIGATES' | 'DERIVED_FROM';
  weight: number;
  created_at: string;
}

export interface WorkspaceSnapshot {
  id: number;
  project_id: number;
  active_file_path: string;
  cursor_line: number;
  cursor_column: number;
  open_tabs_json: string; // Serialized file paths array
  captured_at: string;
}

export interface ProjectSnapshot {
  id: number;
  project_id: number;
  snapshot_summary: string;
  workspace_state_json: string;
  generated_at: string;
}

export interface ContinuationClaim {
  id: number;
  snapshot_id: number;
  claim_text: string;
  confidence_score: number;
}

export interface ContinuationClaimSource {
  claim_id: number;
  node_id: number;
}

export interface TelemetryLog {
  id?: number;
  ids?: number[];
  is_grouped?: boolean;
  group_count?: number;
  group_domain?: string;
  sub_items?: {
    id: number;
    timestamp: string;
    title: string;
    url: string;
  }[];
  timestamp: string;
  raw_timestamp: string;
  category: 'FS_MONITOR' | 'BROWSER_MONITOR' | 'TERMINAL_MONITOR' | 'IDE_MONITOR' | 'COMMUNICATION_MONITOR' | 'APP_MONITOR' | 'COGNITIVE_TRACE' | 'RECOVERY_PLAN' | 'SYSTEM';
  event_type: 'FILE_EDIT' | 'TAB_FOCUS' | 'TERMINAL_FOCUS' | 'IDE_FOCUS' | 'COMMUNICATION_FOCUS' | 'APP_FOCUS' | 'BROWSER_SEARCH' | 'CHECKPOINT' | 'RECOVERY_PLAN' | 'INIT' | 'SYSTEM' | 'DAEMON' | 'COMPACTED';
  display_name: string;
  detail: string;
}
