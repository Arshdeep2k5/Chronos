/**
 * @license
 * SPDX-License-Identifier: Apache-2.0
 */

import { ChronosEvent } from '../types';
import { StabilityState } from './types';

export function computeStability(
  events: ChronosEvent[],
  streamLagMs: number,
  droppedCount: number,
  latencyHistory: any[]
): StabilityState {
  const now = new Date();
  const oneMinAgo = new Date(now.getTime() - 60 * 1000);

  // 1. Count warnings in the last minute
  const minuteWarnings = events.filter(e => {
    const t = new Date(e.timestamp);
    return t >= oneMinAgo && (
      e.event_type === 'TickPerformanceWarning' || 
      e.event_type === 'DroppedFrameDetected' ||
      e.event_type === 'ActionFailed'
    );
  }).length;

  // 2. Base stability score starting at 1.0
  let confidenceScore = 1.0;

  // Deduct for dropped frames (max 0.4 deduction)
  confidenceScore -= Math.min(0.4, droppedCount * 0.08);

  // Deduct for performance warnings (max 0.4 deduction)
  confidenceScore -= Math.min(0.4, minuteWarnings * 0.1);

  // Deduct for SSE lag (max 0.3 deduction)
  if (streamLagMs > 50) {
    const lagOver = streamLagMs - 50;
    confidenceScore -= Math.min(0.3, lagOver * 0.002);
  }

  // Bound between 0.0 and 1.0
  confidenceScore = Math.max(0.0, Math.min(1.0, parseFloat(confidenceScore.toFixed(3))));

  // 3. Compute Drift Velocity (rate of change of delivery lag in ms/tick)
  let driftVelocity = 0;
  if (latencyHistory && latencyHistory.length >= 2) {
    const latest = latencyHistory[latencyHistory.length - 1];
    const prev = latencyHistory[latencyHistory.length - 2];
    if (latest && prev && latest.network_delivery_lag_ms !== undefined && prev.network_delivery_lag_ms !== undefined) {
      driftVelocity = latest.network_delivery_lag_ms - prev.network_delivery_lag_ms;
    }
  }

  // 4. Instability Signal Classification
  let instabilitySignal: StabilityState['instabilitySignal'] = 'STABLE';
  if (confidenceScore <= 0.55 || Math.abs(driftVelocity) > 40) {
    instabilitySignal = 'VOLATILE';
  } else if (confidenceScore <= 0.85 || Math.abs(driftVelocity) > 10) {
    instabilitySignal = 'DRIFTING';
  }

  return {
    confidenceScore,
    driftVelocity,
    instabilitySignal
  };
}
