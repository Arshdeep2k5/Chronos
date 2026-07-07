/**
 * @license
 * SPDX-License-Identifier: Apache-2.0
 */

export interface SystemIntentState {
  intentLabel: 'IDLE' | 'EXPLORING' | 'EXECUTING' | 'RECOVERING' | 'CRITICAL';
  description: string;
  intensity: number; // 0.0 to 1.0
}

export interface CognitiveLoadEstimate {
  eventProcessingRate: number; // events/sec
  tickDensity: number; // ticks/min
  reasoningComplexityScore: number; // 0.0 to 1.0
  loadLevel: 'LOW' | 'MEDIUM' | 'HIGH' | 'CRITICAL';
}

export interface ExecutionPressureSignal {
  hasPressure: boolean;
  criticalPathDelayMs: number;
  backlogSize: number;
  signalLevel: 'NORMAL' | 'WARN' | 'CRITICAL';
}

export interface StabilityState {
  confidenceScore: number; // 0.0 to 1.0
  driftVelocity: number; // ms/sec of lag change
  instabilitySignal: 'STABLE' | 'DRIFTING' | 'VOLATILE';
}

export interface IntentGraphNode {
  id: string;
  type: 'TickFrame' | 'Warning' | 'ACK' | 'Risk' | 'Commitment';
  label: string;
  timestamp: string;
}

export interface IntentGraphEdge {
  source: string;
  target: string;
  relationship: string;
}

export interface IntentGraph {
  nodes: IntentGraphNode[];
  edges: IntentGraphEdge[];
}

export interface CognitiveState {
  intent: SystemIntentState;
  load: CognitiveLoadEstimate;
  pressure: ExecutionPressureSignal;
  stability: StabilityState;
  graph: IntentGraph;
  timestamp: string;
}
