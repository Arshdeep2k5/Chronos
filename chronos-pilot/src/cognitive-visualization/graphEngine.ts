/**
 * @license
 * SPDX-License-Identifier: Apache-2.0
 */

import { IntentGraph } from '../cognitive/types';
import { GraphState, VisualNode, VisualEdge } from './graphTypes';

export function transformGraph(
  intentGraph: IntentGraph,
  stabilityScore: number,
  backlogSize: number
): GraphState {
  // 1. Transform Nodes
  const nodes: VisualNode[] = intentGraph.nodes.map(node => {
    let color = '#00A3FF'; // Default blue
    let size = 12;
    let pulse = false;

    switch (node.type) {
      case 'TickFrame':
        // Color depends on stability score
        color = stabilityScore > 0.8 ? '#00E575' : stabilityScore > 0.55 ? '#FF8C00' : '#FF0055';
        size = 16;
        pulse = stabilityScore <= 0.55;
        break;
      case 'Warning':
        color = '#FF0055'; // Intense Red
        size = 14;
        pulse = true;
        break;
      case 'ACK':
        color = '#00FFFF'; // Cyan
        size = 10;
        break;
      case 'Risk':
        color = '#FF8C00'; // Orange
        size = 14;
        break;
      case 'Commitment':
        color = '#9F00FF'; // Purple
        size = 12;
        // Pulse if pressure is high
        pulse = backlogSize > 3;
        break;
    }

    return {
      id: node.id,
      type: node.type,
      label: node.label,
      x: 0,
      y: 0,
      vx: 0,
      vy: 0,
      size,
      color,
      pulse,
      metadata: {
        timestamp: node.timestamp,
        derivedInfo: `CSL derived node of type ${node.type}`
      }
    };
  });

  // 2. Transform Edges
  const edges: VisualEdge[] = intentGraph.edges.map((edge, idx) => {
    let style: VisualEdge['style'] = 'thin';
    let color = '#444444'; // Default dark gray

    switch (edge.relationship) {
      case 'TRIGGERS_ACK':
        style = 'solid';
        color = '#00FFFF';
        break;
      case 'CONTAINS_WARNING':
        style = 'dashed';
        color = '#FF0055';
        break;
      case 'EVALUATES_RISK':
        style = 'solid';
        color = '#FF8C00';
        break;
      case 'TEMPORAL_ADJACENT':
        style = 'thin';
        color = '#333333';
        break;
    }

    return {
      id: `edge-${idx}`,
      source: edge.source,
      target: edge.target,
      relationship: edge.relationship,
      style,
      color
    };
  });

  return { nodes, edges };
}
