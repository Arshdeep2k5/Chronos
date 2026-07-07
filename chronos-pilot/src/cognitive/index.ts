/**
 * @license
 * SPDX-License-Identifier: Apache-2.0
 */

import { ChronosEvent } from '../types';
import { CognitiveState } from './types';
import { interpretSemantics } from './semanticInterpreter';
import { computeStability } from './stabilityModel';
import { buildIntentGraph } from './intentGraph';

export * from './types';

export function computeCognitiveState(
  events: ChronosEvent[],
  streamLagMs: number,
  droppedCount: number,
  latencyHistory: any[]
): CognitiveState {
  const { intent, load, pressure } = interpretSemantics(events);
  const stability = computeStability(events, streamLagMs, droppedCount, latencyHistory);
  const graph = buildIntentGraph(events);

  return {
    intent,
    load,
    pressure,
    stability,
    graph,
    timestamp: new Date().toISOString()
  };
}
