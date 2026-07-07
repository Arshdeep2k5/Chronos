/**
 * @license
 * SPDX-License-Identifier: Apache-2.0
 */

import { ChronosEvent } from '../types';
import { SystemIntentState, CognitiveLoadEstimate, ExecutionPressureSignal } from './types';

export function interpretSemantics(events: ChronosEvent[]): {
  intent: SystemIntentState;
  load: CognitiveLoadEstimate;
  pressure: ExecutionPressureSignal;
} {
  const now = new Date();
  const thirtySecsAgo = new Date(now.getTime() - 30 * 1000);
  const oneMinAgo = new Date(now.getTime() - 60 * 1000);

  // 1. Filter recent events
  const recentEvents = events.filter(e => {
    const t = new Date(e.timestamp);
    return t >= thirtySecsAgo;
  });

  const minuteEvents = events.filter(e => {
    const t = new Date(e.timestamp);
    return t >= oneMinAgo;
  });

  // 2. Count types
  const tickFrames = minuteEvents.filter(e => e.event_type === 'TickFrameEmitted');
  const warnings = minuteEvents.filter(e => 
    e.event_type === 'TickPerformanceWarning' || 
    e.event_type === 'ActionFailed' || 
    e.event_type === 'ExecutionFailed'
  );

  const actionStarted = events.filter(e => e.event_type === 'ActionStarted' || e.event_type === 'ExecutionStarted');
  const actionEnded = events.filter(e => e.event_type === 'ActionCompleted' || e.event_type === 'ExecutionCompleted' || e.event_type === 'ActionFailed' || e.event_type === 'ExecutionFailed');
  const activeActionsCount = Math.max(0, actionStarted.length - actionEnded.length);

  const commitmentsDiscovered = events.filter(e => e.event_type === 'CommitmentDiscovered');
  const commitmentsCompleted = events.filter(e => e.event_type === 'CommitmentCompleted');
  const openCommitmentsCount = Math.max(0, commitmentsDiscovered.length - commitmentsCompleted.length);

  // ─── Derive SystemIntentState ──────────────────────────────────────────────
  let intentLabel: SystemIntentState['intentLabel'] = 'IDLE';
  let description = 'System is quiescent, awaiting environmental stimulations.';
  let intensity = 0.1;

  if (warnings.length > 3) {
    intentLabel = 'CRITICAL';
    description = 'High frequency of execution warnings or failures. Operational integrity threatened.';
    intensity = 0.95;
  } else if (warnings.length > 0) {
    intentLabel = 'RECOVERING';
    description = 'Mitigating recently detected tick warnings or execution performance degradation.';
    intensity = 0.7;
  } else if (activeActionsCount > 0) {
    intentLabel = 'EXECUTING';
    description = 'Active execution pipeline engaged. Dispatching commands and verifying outcomes.';
    intensity = 0.85;
  } else if (recentEvents.some(e => e.event_type === 'FileModified' || e.event_type === 'BrowserTabFocused' || e.event_type === 'CommitmentDiscovered')) {
    intentLabel = 'EXPLORING';
    description = 'Scanning file workspace modifications and constructing cognitive commitments.';
    intensity = 0.5;
  }

  const intent: SystemIntentState = { intentLabel, description, intensity };

  // ─── Derive CognitiveLoadEstimate ─────────────────────────────────────────
  const eventProcessingRate = parseFloat((recentEvents.length / 30).toFixed(2));
  const tickDensity = tickFrames.length;

  // Reasoning complexity: open commitments scale complexity
  const reasoningComplexityScore = Math.min(1.0, parseFloat((openCommitmentsCount * 0.15 + activeActionsCount * 0.25).toFixed(2)));

  let loadLevel: CognitiveLoadEstimate['loadLevel'] = 'LOW';
  if (reasoningComplexityScore > 0.8 || eventProcessingRate > 5.0) {
    loadLevel = 'CRITICAL';
  } else if (reasoningComplexityScore > 0.5 || eventProcessingRate > 2.0) {
    loadLevel = 'HIGH';
  } else if (reasoningComplexityScore > 0.2) {
    loadLevel = 'MEDIUM';
  }

  const load: CognitiveLoadEstimate = {
    eventProcessingRate,
    tickDensity,
    reasoningComplexityScore,
    loadLevel
  };

  // ─── Derive ExecutionPressureSignal ────────────────────────────────────────
  let hasPressure = false;
  let criticalPathDelayMs = 0;

  // Extract latency from the latest TickFrameEmitted event payload if present
  const latestFrameEvent = [...events].reverse().find(e => e.event_type === 'TickFrameEmitted');
  if (latestFrameEvent && latestFrameEvent.payload) {
    const telemetry = latestFrameEvent.payload.telemetry;
    if (telemetry) {
      criticalPathDelayMs = telemetry.total_duration_ms || 0;
      if (criticalPathDelayMs > 50) {
        hasPressure = true;
      }
    }
  }

  let signalLevel: ExecutionPressureSignal['signalLevel'] = 'NORMAL';
  if (hasPressure && openCommitmentsCount > 5) {
    signalLevel = 'CRITICAL';
  } else if (hasPressure || openCommitmentsCount > 2) {
    signalLevel = 'WARN';
  }

  const pressure: ExecutionPressureSignal = {
    hasPressure,
    criticalPathDelayMs,
    backlogSize: openCommitmentsCount,
    signalLevel
  };

  return { intent, load, pressure };
}
