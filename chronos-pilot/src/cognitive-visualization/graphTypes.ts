/**
 * @license
 * SPDX-License-Identifier: Apache-2.0
 */

export interface VisualNode {
  id: string;
  type: 'TickFrame' | 'Warning' | 'ACK' | 'Risk' | 'Commitment';
  label: string;
  x: number;
  y: number;
  vx: number;
  vy: number;
  size: number;
  color: string;
  pulse?: boolean;
  metadata?: any;
}

export interface VisualEdge {
  id: string;
  source: string;
  target: string;
  relationship: string;
  style: 'solid' | 'dashed' | 'thin';
  color: string;
}

export interface GraphState {
  nodes: VisualNode[];
  edges: VisualEdge[];
}
