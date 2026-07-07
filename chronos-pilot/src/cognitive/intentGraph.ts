/**
 * @license
 * SPDX-License-Identifier: Apache-2.0
 */

import { ChronosEvent } from '../types';
import { IntentGraph, IntentGraphNode, IntentGraphEdge } from './types';

export function buildIntentGraph(events: ChronosEvent[]): IntentGraph {
  const nodes: IntentGraphNode[] = [];
  const edges: IntentGraphEdge[] = [];

  // Limit processing to latest 25 events to keep CSL processing lightweight and responsive
  const targetEvents = [...events].reverse().slice(0, 25).reverse();

  // 1. Generate Nodes
  targetEvents.forEach((ev, idx) => {
    let nodeType: IntentGraphNode['type'] | null = null;
    let label = ev.event_type;

    if (ev.event_type === 'TickFrameEmitted') {
      nodeType = 'TickFrame';
      const seq = ev.payload?.telemetry?.tick_sequence;
      label = `TickFrame #${seq !== undefined ? seq : idx}`;
    } else if (ev.event_type === 'TickPerformanceWarning' || ev.event_type === 'ActionFailed' || ev.event_type === 'ExecutionFailed') {
      nodeType = 'Warning';
      label = ev.event_type === 'TickPerformanceWarning' ? 'Slow Tick Warning' : 'Execution Failure';
    } else if (ev.event_type === 'UiTelemetryAckReceived') {
      nodeType = 'ACK';
      label = 'UI Render ACK';
    } else if (ev.event_type === 'RiskForecastResolved') {
      nodeType = 'Risk';
      label = 'Risk Assessment';
    } else if (ev.event_type === 'CommitmentDiscovered' || ev.event_type === 'CommitmentCompleted') {
      nodeType = 'Commitment';
      label = ev.event_type === 'CommitmentDiscovered' ? 'Commitment Discovered' : 'Commitment Resolved';
    }

    if (nodeType) {
      nodes.push({
        id: ev.id ? String(ev.id) : `node-${idx}`,
        type: nodeType,
        label,
        timestamp: ev.timestamp
      });
    }
  });

  // 2. Generate Causal & Temporal Edges
  for (let i = 0; i < nodes.length; i++) {
    const current = nodes[i];

    // Temporal Adjacency
    if (i < nodes.length - 1) {
      const next = nodes[i + 1];
      edges.push({
        source: current.id,
        target: next.id,
        relationship: 'TEMPORAL_ADJACENT'
      });
    }

    // Heuristic Causality
    if (current.type === 'ACK') {
      // Find matching TickFrame preceding this ACK
      const matchingTick = [...nodes]
        .slice(0, i)
        .reverse()
        .find(n => n.type === 'TickFrame');
      if (matchingTick) {
        edges.push({
          source: matchingTick.id,
          target: current.id,
          relationship: 'TRIGGERS_ACK'
        });
      }
    }

    if (current.type === 'Warning') {
      // Warnings are caused by the active TickFrame
      const matchingTick = [...nodes]
        .slice(0, i)
        .reverse()
        .find(n => n.type === 'TickFrame');
      if (matchingTick) {
        edges.push({
          source: matchingTick.id,
          target: current.id,
          relationship: 'CONTAINS_WARNING'
        });
      }
    }

    if (current.type === 'Risk') {
      // Risk is derived from active Commitments
      const matchingCommitment = [...nodes]
        .slice(0, i)
        .reverse()
        .find(n => n.type === 'Commitment');
      if (matchingCommitment) {
        edges.push({
          source: matchingCommitment.id,
          target: current.id,
          relationship: 'EVALUATES_RISK'
        });
      }
    }
  }

  return { nodes, edges };
}
