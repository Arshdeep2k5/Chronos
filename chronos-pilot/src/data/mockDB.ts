/**
 * @license
 * SPDX-License-Identifier: Apache-2.0
 */

import fs from 'fs';
import path from 'path';
import {
  Project,
  ProjectState,
  ContextNode,
  ContextEvent,
  BrowserSession,
  SearchQuery,
  Commitment,
  ProjectDeadline,
  ProjectAction,
  ProjectCheckpoint,
  RecoveryPlan,
  AutonomousResearchBrief,
  GraphEdge,
  WorkspaceSnapshot,
  DeadLetterQueueItem
} from '../types';

const DB_FILE_PATH = path.join(process.cwd(), 'chronos_local_db.json');

export class ChronosDatabase {
  projects: Project[] = [];
  projectState: ProjectState[] = [];
  contextNodes: ContextNode[] = [];
  contextEvents: ContextEvent[] = [];
  browserSessions: BrowserSession[] = [];
  searchQueries: SearchQuery[] = [];
  commitments: Commitment[] = [];
  projectDeadlines: ProjectDeadline[] = [];
  projectActions: ProjectAction[] = [];
  projectCheckpoints: ProjectCheckpoint[] = [];
  recoveryPlans: RecoveryPlan[] = [];
  autonomousResearchBriefs: AutonomousResearchBrief[] = [];
  graphEdges: GraphEdge[] = [];
  workspaceSnapshots: WorkspaceSnapshot[] = [];
  deadLetterQueue: DeadLetterQueueItem[] = [];

  constructor() {
    this.load();
    if (this.projects.length === 0) {
      this.seedInitialData();
    }
  }

  load() {
    try {
      if (fs.existsSync(DB_FILE_PATH)) {
        const raw = fs.readFileSync(DB_FILE_PATH, 'utf-8');
        const data = JSON.parse(raw);
        this.projects = data.projects || [];
        this.projectState = data.projectState || [];
        this.contextNodes = data.contextNodes || [];
        this.contextEvents = data.contextEvents || [];
        this.browserSessions = data.browserSessions || [];
        this.searchQueries = data.searchQueries || [];
        this.commitments = data.commitments || [];
        this.projectDeadlines = data.projectDeadlines || [];
        this.projectActions = data.projectActions || [];
        this.projectCheckpoints = data.projectCheckpoints || [];
        this.recoveryPlans = data.recoveryPlans || [];
        this.autonomousResearchBriefs = data.autonomousResearchBriefs || [];
        this.graphEdges = data.graphEdges || [];
        this.workspaceSnapshots = data.workspaceSnapshots || [];
        this.deadLetterQueue = data.deadLetterQueue || [];
      }
    } catch (e) {
      console.error('Error loading Chronos Database:', e);
    }
  }

  save() {
    try {
      const data = {
        projects: this.projects,
        projectState: this.projectState,
        contextNodes: this.contextNodes,
        contextEvents: this.contextEvents,
        browserSessions: this.browserSessions,
        searchQueries: this.searchQueries,
        commitments: this.commitments,
        projectDeadlines: this.projectDeadlines,
        projectActions: this.projectActions,
        projectCheckpoints: this.projectCheckpoints,
        recoveryPlans: this.recoveryPlans,
        autonomousResearchBriefs: this.autonomousResearchBriefs,
        graphEdges: this.graphEdges,
        workspaceSnapshots: this.workspaceSnapshots,
        deadLetterQueue: this.deadLetterQueue
      };
      fs.writeFileSync(DB_FILE_PATH, JSON.stringify(data, null, 2), 'utf-8');
    } catch (e) {
      console.error('Error saving Chronos Database:', e);
    }
  }

  seedInitialData() {
    // 1. Seed Projects
    this.projects = [
      { id: 1, project_name: 'ML Coursework Assignment', status: 'ACTIVE', created_at: '2026-06-09T10:00:00Z' },
      { id: 2, project_name: 'Stripe Billing Integration', status: 'ACTIVE', created_at: '2026-05-24T10:00:00Z' }
    ];

    // 2. Seed Project State
    this.projectState = [
      {
        project_id: 1,
        current_summary: 'Developing an end-to-end Retrieval-Augmented Generation (RAG) chunking pipeline for custom academic documents.',
        current_entry_point: 'notebooks/rag_eval.ipynb',
        next_action: 'Train local classifier & run baseline',
        confidence_score: 0.92,
        updated_at: '2026-06-23T10:15:00-07:00'
      },
      {
        project_id: 2,
        current_summary: 'Wiring local API handlers to handle custom Stripe billing plan subscriptions and webhook notifications safely.',
        current_entry_point: 'src/billing/stripe_webhook.ts',
        next_action: 'Implement Stripe signature validation verification',
        confidence_score: 0.88,
        updated_at: '2026-06-23T10:15:00-07:00'
      }
    ];

    // 3. Seed Context Nodes
    this.contextNodes = [
      // ML Coursework Assignment (Project 1)
      { id: 1, project_id: 1, entity_key: 'FILE:notebooks/rag_eval.ipynb', entity_type: 'FILE', display_name: 'rag_eval.ipynb', created_at: '2026-06-09T10:15:00Z' },
      { id: 2, project_id: 1, entity_key: 'FILE:data/assignment_prompt.md', entity_type: 'FILE', display_name: 'assignment_prompt.md', created_at: '2026-06-09T10:20:00Z' },
      { id: 3, project_id: 1, entity_key: 'URL:https://arxiv.org/abs/2005.11401', entity_type: 'URL', display_name: 'Retrieval-Augmented Generation Paper', created_at: '2026-06-15T14:30:00Z' },
      
      // Stripe Billing Integration (Project 2)
      { id: 4, project_id: 2, entity_key: 'FILE:src/billing/stripe_webhook.ts', entity_type: 'FILE', display_name: 'stripe_webhook.ts', created_at: '2026-05-24T11:00:00Z' },
      { id: 5, project_id: 2, entity_key: 'FILE:package.json', entity_type: 'FILE', display_name: 'package.json', created_at: '2026-05-24T10:05:00Z' },
      { id: 6, project_id: 2, entity_key: 'URL:https://docs.stripe.com/webhooks/signatures', entity_type: 'URL', display_name: 'Stripe Webhook Signatures Docs', created_at: '2026-06-20T10:15:00Z' }
    ];

    // 4. Seed Context Events (Faking interaction logs)
    this.contextEvents = [
      // ML Assignment logs (Project 1)
      { id: 1, node_id: 1, event_type: 'EDITED', interaction_duration: 1200, captured_at: '2026-06-20T14:00:00Z' },
      { id: 2, node_id: 2, event_type: 'OPENED', interaction_duration: 300, captured_at: '2026-06-20T14:20:00Z' },
      { id: 3, node_id: 3, event_type: 'TAB_FOCUS', interaction_duration: 900, captured_at: '2026-06-21T09:30:00Z' },

      // Stripe Billing logs (Project 2)
      { id: 4, node_id: 4, event_type: 'EDITED', interaction_duration: 2400, captured_at: '2026-06-20T11:00:00Z' },
      { id: 5, node_id: 6, event_type: 'TAB_FOCUS', interaction_duration: 1500, captured_at: '2026-06-20T11:40:00Z' }
    ];

    // 5. Seed Commitments
    this.commitments = [
      {
        id: 1,
        project_id: 1,
        title: 'Machine Learning Coursework',
        commitment_type: 'ASSIGNMENT',
        deadline_date: '2026-07-15T23:59:59Z', // ~22 days remaining
        confidence_score: 0.92,
        source_node_id: 2,
        status: 'OPEN',
        created_at: '2026-06-09T10:20:00Z'
      },
      {
        id: 2,
        project_id: 2,
        title: 'Stripe Webhook Verification Release',
        commitment_type: 'DELIVERABLE',
        deadline_date: '2026-06-25T18:00:00Z', // ~2 days remaining!
        confidence_score: 0.88,
        source_node_id: 6,
        status: 'OPEN',
        created_at: '2026-05-24T11:00:00Z'
      }
    ];

    // 6. Seed Project Deadlines
    this.projectDeadlines = [
      { id: 1, project_id: 1, deadline_label: 'ML Assignment Final Submission', target_date: '2026-07-15T23:59:59Z', importance_tier: 'HIGH', created_at: '2026-06-09T10:00:00Z' },
      { id: 2, project_id: 2, deadline_label: 'Stripe Billing Production Deployment', target_date: '2026-06-25T18:00:00Z', importance_tier: 'CRITICAL', created_at: '2026-05-24T10:00:00Z' }
    ];

    // 7. Seed Project Actions
    this.projectActions = [
      // ML course actions
      { id: 1, project_id: 1, action_text: 'Complete literature review & parse inputs', estimated_effort_hours: 3.5, status: 'PENDING', priority_score: 0.0, created_at: '2026-06-09T10:30:00Z' },
      { id: 2, project_id: 1, action_text: 'Train local classifier & run baseline', estimated_effort_hours: 4.0, status: 'PENDING', priority_score: 0.0, created_at: '2026-06-09T10:30:00Z' },
      { id: 3, project_id: 1, action_text: 'Finalize report writing & export PDFs', estimated_effort_hours: 2.5, status: 'PENDING', priority_score: 0.0, created_at: '2026-06-09T10:30:00Z' },

      // Stripe actions
      { id: 4, project_id: 2, action_text: 'Implement Stripe signature validation verification', estimated_effort_hours: 5.5, status: 'PENDING', priority_score: 0.0, created_at: '2026-05-24T11:15:00Z' },
      { id: 5, project_id: 2, action_text: 'Test webhooks locally using Stripe CLI sandbox', estimated_effort_hours: 2.0, status: 'PENDING', priority_score: 0.0, created_at: '2026-05-24T11:15:00Z' }
    ];

    // 8. Seed Checkpoints
    this.projectCheckpoints = [
      {
        id: 1,
        project_id: 1,
        accomplished_text: 'Parsed raw prompt guidelines and set up a basic token-splitter notebook.',
        blocked_text: 'Experiencing unstable index retrieval and high latency during multi-vector search database loops.',
        next_steps_text: 'Investigate alternative chunking/splitting strategies and evaluate embeddings on local test datasets.',
        created_at: '2026-06-20T15:00:00Z'
      },
      {
        id: 2,
        project_id: 2,
        accomplished_text: 'Implemented basic express server routing to receive Stripe JSON post webhook payloads.',
        blocked_text: 'Stuck trying to mock Stripe webhook signatures locally to test verification middleware safely without calling remote servers.',
        next_steps_text: 'Find documentation or solutions for verifying webhooks in local offline test suites.',
        created_at: '2026-06-20T12:00:00Z'
      }
    ];

    // 9. Seed Graph Edges
    this.graphEdges = [
      { source_node_id: 1, target_node_id: 2, edge_type: 'REFERENCES', weight: 1.5, created_at: '2026-06-09T10:20:00Z' },
      { source_node_id: 1, target_node_id: 3, edge_type: 'DERIVED_FROM', weight: 1.0, created_at: '2026-06-15T14:30:00Z' },
      { source_node_id: 4, target_node_id: 6, edge_type: 'INVESTIGATES', weight: 2.0, created_at: '2026-06-20T11:00:00Z' }
    ];

    // 10. Seed Workspace Snapshots (Restore targets)
    this.workspaceSnapshots = [
      {
        id: 1,
        project_id: 1,
        active_file_path: 'notebooks/rag_eval.ipynb',
        cursor_line: 15,
        cursor_column: 2,
        open_tabs_json: JSON.stringify(['notebooks/rag_eval.ipynb', 'data/assignment_prompt.md']),
        captured_at: '2026-06-20T14:30:00Z'
      },
      {
        id: 2,
        project_id: 2,
        active_file_path: 'src/billing/stripe_webhook.ts',
        cursor_line: 42,
        cursor_column: 12,
        open_tabs_json: JSON.stringify(['package.json', 'src/billing/stripe_webhook.ts']),
        captured_at: '2026-06-20T11:45:00Z'
      }
    ];

    // 11. Seed Browser Sessions
    this.browserSessions = [
      {
        id: 1,
        project_id: 2,
        url: 'https://docs.stripe.com/webhooks/signatures',
        page_title: 'Webhook Signatures | Stripe Documentation',
        domain: 'docs.stripe.com',
        visit_started_at: '2026-06-20T11:10:00Z',
        visit_ended_at: '2026-06-20T11:35:00Z',
        active_seconds: 1500,
        created_at: '2026-06-20T11:10:00Z'
      }
    ];

    // 12. Seed Search Queries
    this.searchQueries = [
      {
        id: 1,
        browser_session_id: 1,
        query_text: 'stripe webhook local signature test failure',
        created_at: '2026-06-20T11:12:00Z'
      }
    ];

    this.save();
  }

  // --- MATHEMATICAL MATH MODELING ENGINES ---

  /**
   * Evaluates and updates dynamic Priority Score (AP) for every uncompleted action.
   * AP(A_k, P_j) = w_prox * P_prox + w_attn * W_attn_avg + w_urg * C_urg + w_effort * E_achieve
   */
  computeActionPriorities(currentTimeStr: string = '2026-06-23T10:15:00-07:00'): ProjectAction[] {
    const currentTime = new Date(currentTimeStr).getTime();
    const horizonMs = 7 * 24 * 60 * 60 * 1000; // 7 days (168 hours)

    const w_prox = 0.40;
    const w_attn = 0.20;
    const w_urg = 0.25;
    const w_effort = 0.15;

    return this.projectActions.map(action => {
      if (action.status !== 'PENDING') {
        return action;
      }

      // 1. Proximity Metric (P_prox)
      const projectDeadline = this.projectDeadlines.find(d => d.project_id === action.project_id);
      let p_prox = 0;
      let hoursRemaining = 168;

      if (projectDeadline) {
        const deadlineTime = new Date(projectDeadline.target_date).getTime();
        const timeRemainingMs = deadlineTime - currentTime;
        hoursRemaining = Math.max(0, timeRemainingMs / (60 * 60 * 1000));
        p_prox = Math.max(0, 1 - (timeRemainingMs / horizonMs));
      }

      // 2. Average Attention Weight (W_attn_avg)
      // Retrieve nodes for project and compute W_attn
      const projectNodes = this.contextNodes.filter(n => n.project_id === action.project_id);
      let totalAttnWeight = 0;

      projectNodes.forEach(node => {
        // W_attn(C_i) = 0.50 * log(1 + T_focus) + 0.35 * N_edits + 0.15 * N_revisits
        const events = this.contextEvents.filter(e => e.node_id === node.id);
        const focusEvents = events.filter(e => e.event_type === 'TAB_FOCUS' || e.event_type === 'OPENED' || e.event_type === 'EDITED');
        const focusSeconds = focusEvents.reduce((acc, ev) => acc + (ev.interaction_duration || 0), 0);
        
        const n_edits = events.filter(e => e.event_type === 'EDITED').length;
        const n_revisits = events.length; // Approximate revisits with event counts

        const w_attn_node = 0.50 * Math.log1p(focusSeconds) + 0.35 * n_edits + 0.15 * n_revisits;
        totalAttnWeight += w_attn_node;
      });

      const avg_attn = projectNodes.length > 0 ? (totalAttnWeight / projectNodes.length) : 0;
      // Normalize avg_attn to a 0-1 scale for calculation (log scaling caps nicely)
      const avg_attn_norm = Math.min(1.0, avg_attn / 5.0);

      // 3. Binary Urgency Flag (C_urg)
      // Urgency flag is 1 if marked as next steps / blockers in checkpoint
      const latestCheckpoint = this.projectCheckpoints
        .filter(c => c.project_id === action.project_id)
        .sort((a, b) => new Date(b.created_at).getTime() - new Date(a.created_at).getTime())[0];

      let c_urg = 0;
      if (latestCheckpoint) {
        const isNextStep = latestCheckpoint.next_steps_text?.toLowerCase().includes(action.action_text.toLowerCase());
        const isBlocker = latestCheckpoint.blocked_text?.toLowerCase().includes(action.action_text.toLowerCase());
        if (isNextStep || isBlocker) {
          c_urg = 1.0;
        }
      }

      // 4. Effort Achievability Metric (E_achieve)
      // Shorter tasks receive priority boost near deadline
      const effortHours = action.estimated_effort_hours;
      const horizonHours = 168; // 7 days
      const e_achieve = 1.0 - (Math.min(effortHours, horizonHours) / horizonHours);

      // Compute raw Action Priority score on 0-10 scale
      const rawAP = (w_prox * p_prox) + (w_attn * avg_attn_norm) + (w_urg * c_urg) + (w_effort * e_achieve);
      action.priority_score = parseFloat((rawAP * 10).toFixed(2));

      // 5. Generate human-readable "Why Now?" diagnostics
      const diagnostics: string[] = [];
      if (w_prox * p_prox >= 0.20) {
        const days = Math.max(0.1, hoursRemaining / 24).toFixed(1);
        diagnostics.push(`Deadline is imminent (under ${days} days remaining)`);
      }
      if (c_urg === 1.0) {
        diagnostics.push(`Explicitly flagged or referenced in your recent project checkpoints`);
      }
      if (avg_attn_norm <= 0.25) {
        diagnostics.push(`Project progress has stalled (zero focus detected recently)`);
      }
      if (w_effort * e_achieve >= 0.08) {
        diagnostics.push(`Quick win: highly achievable task (estimated ${effortHours} hrs)`);
      }
      if (diagnostics.length === 0) {
        diagnostics.push(`Standard scheduled sequencing of commitments`);
      }
      action.why_now_diagnostics = diagnostics;

      return action;
    });
  }

  /**
   * Computes Commitment Health score
   * Health(C) = w_prog * Prog + w_act * Act + w_time * T_norm - w_eff * E_norm
   */
  getCommitmentHealth(commitment: Commitment, currentTimeStr: string = '2026-06-23T10:15:00-07:00'): 'GREEN' | 'YELLOW' | 'RED' {
    if (commitment.status !== 'OPEN') return 'GREEN';

    const currentTime = new Date(currentTimeStr).getTime();
    
    // 1. Progress Metric: Completed / Total actions
    const projectActions = this.projectActions.filter(a => a.project_id === commitment.project_id);
    const completedActions = projectActions.filter(a => a.status === 'COMPLETED').length;
    const totalActions = projectActions.length;
    const prog = totalActions > 0 ? (completedActions / totalActions) : 1.0;

    // 2. Activity Metric (Act_rec): Normalized focus seconds over past 48 hours
    const nodes = this.contextNodes.filter(n => n.project_id === commitment.project_id);
    let recentFocusSec = 0;
    const fortyEightHoursAgo = currentTime - (48 * 60 * 60 * 1000);

    nodes.forEach(node => {
      const events = this.contextEvents.filter(e => e.node_id === node.id);
      events.forEach(e => {
        const evTime = new Date(e.captured_at).getTime();
        if (evTime >= fortyEightHoursAgo) {
          recentFocusSec += (e.interaction_duration || 0);
        }
      });
    });
    // Normalize to 4 hours of maximum expected active focus (14400 sec)
    const act_rec = Math.min(1.0, recentFocusSec / 14400);

    // 3. Time Metric (T_norm)
    let t_norm = 1.0;
    let timeRemainingHours = 168;
    if (commitment.deadline_date) {
      const deadlineTime = new Date(commitment.deadline_date).getTime();
      const timeRemainingMs = deadlineTime - currentTime;
      timeRemainingHours = Math.max(0, timeRemainingMs / (60 * 60 * 1000));
      t_norm = Math.min(1.0, timeRemainingHours / 168); // 7-day planning ceiling
    }

    // 4. Effort remaining Metric (E_norm)
    const pendingActions = projectActions.filter(a => a.status === 'PENDING');
    const estimatedRemainingHours = pendingActions.reduce((sum, a) => sum + a.estimated_effort_hours, 0);
    const e_norm = Math.min(1.0, estimatedRemainingHours / 168);

    const w_prog = 0.30;
    const w_act = 0.25;
    const w_time = 0.25;
    const w_eff = 0.20;

    const healthVal = (w_prog * prog) + (w_act * act_rec) + (w_time * t_norm) - (w_eff * e_norm);

    if (healthVal >= 0.70) return 'GREEN';
    if (healthVal >= 0.40) return 'YELLOW';
    return 'RED';
  }

  /**
   * Computes Risk score (estimated remaining effort / available historical capacity)
   * Risk(C) = Remaining_Effort / Available_Capacity
   */
  getCommitmentRisk(commitment: Commitment, currentTimeStr: string = '2026-06-23T10:15:00-07:00'): {
    risk_score: number,
    completion_chance: number,
    simulated_failure_date: string,
    marignal_loss_24h: number
  } {
    const currentTime = new Date(currentTimeStr).getTime();
    
    // 1. Sum of remaining effort hours
    const projectActions = this.projectActions.filter(a => a.project_id === commitment.project_id);
    const pendingActions = projectActions.filter(a => a.status === 'PENDING');
    const effortRemaining = pendingActions.reduce((sum, a) => sum + a.estimated_effort_hours, 0);

    // 2. Historical daily focused capacity (simulating 1.5 focused hours/day)
    const dailyCapacityHours = 2.0;

    // 3. Available days until deadline
    let daysRemaining = 7;
    if (commitment.deadline_date) {
      const deadlineTime = new Date(commitment.deadline_date).getTime();
      daysRemaining = Math.max(0.1, (deadlineTime - currentTime) / (24 * 60 * 60 * 1000));
    }

    const availableHoursWindow = daysRemaining * dailyCapacityHours;

    // Risk coefficient is remaining effort divided by available capacity
    const riskScore = parseFloat(Math.min(2.0, effortRemaining / Math.max(0.1, availableHoursWindow)).toFixed(2));

    // Completion chance computed via logistic function: P_comp(t) = 1 / (1 + e^(10 * (Risk - 1.0)))
    const completionChance = parseFloat((1 / (1 + Math.exp(6 * (riskScore - 1.0)))).toFixed(2));

    // Simulated failure date calculation
    const daysNeeded = effortRemaining / dailyCapacityHours;
    const simulatedSuccessTime = currentTime + (daysNeeded * 24 * 60 * 60 * 1000);
    const simulatedFailureDate = new Date(simulatedSuccessTime).toISOString().split('T')[0];

    // Marginal drop calculation: complete consequence simulator projection
    // P_comp(t + 24) = P_comp(t) * e^(-0.15 * (24 / hoursRemaining))
    const hoursRemaining = daysRemaining * 24;
    const delayFactor = Math.exp(-0.15 * (24 / Math.max(1, hoursRemaining)));
    const delayedChance = parseFloat((completionChance * delayFactor).toFixed(2));
    const marginalLoss = parseFloat((completionChance - delayedChance).toFixed(2));

    return {
      risk_score: riskScore,
      completion_chance: completionChance,
      simulated_failure_date: simulatedFailureDate,
      marignal_loss_24h: marginalLoss
    };
  }
}
