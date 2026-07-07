/**
 * @license
 * SPDX-License-Identifier: Apache-2.0
 */

import { GraphState, VisualNode } from './graphTypes';

const REPULSION = 800;
const SPRING_STIFFNESS = 0.04;
const REST_LENGTH = 80;
const CENTER_GRAVITY = 0.02;
const DAMPING = 0.82;

export function initializeNodePositions(nodes: VisualNode[], width: number, height: number): VisualNode[] {
  return nodes.map((node, idx) => {
    // If node already has positions, preserve them
    if (node.x !== 0 || node.y !== 0) {
      return node;
    }
    // Otherwise place them radially around center
    const angle = (idx / nodes.length) * 2 * Math.PI;
    const radius = 100 + Math.random() * 50;
    return {
      ...node,
      x: width / 2 + Math.cos(angle) * radius,
      y: height / 2 + Math.sin(angle) * radius,
      vx: 0,
      vy: 0
    };
  });
}

export function stepSimulation(state: GraphState, width: number, height: number): GraphState {
  const nodes = [...state.nodes];
  const edges = state.edges;

  // 1. Initialize forces/velocities
  for (const node of nodes) {
    node.vx = node.vx || 0;
    node.vy = node.vy || 0;
  }

  // 2. Electrostatic Repulsion (Coulomb's Law)
  for (let i = 0; i < nodes.length; i++) {
    for (let j = i + 1; j < nodes.length; j++) {
      const nodeA = nodes[i];
      const nodeB = nodes[j];

      const dx = nodeB.x - nodeA.x;
      const dy = nodeB.y - nodeA.y;
      const distSq = dx * dx + dy * dy + 0.1;
      const dist = Math.sqrt(distSq);

      if (dist < 250) {
        const force = REPULSION / distSq;
        const fx = (dx / dist) * force;
        const fy = (dy / dist) * force;

        nodeA.vx -= fx;
        nodeA.vy -= fy;
        nodeB.vx += fx;
        nodeB.vy += fy;
      }
    }
  }

  // 3. Spring Tension (Hooke's Law)
  for (const edge of edges) {
    const nodeA = nodes.find(n => n.id === edge.source);
    const nodeB = nodes.find(n => n.id === edge.target);

    if (nodeA && nodeB) {
      const dx = nodeB.x - nodeA.x;
      const dy = nodeB.y - nodeA.y;
      const dist = Math.sqrt(dx * dx + dy * dy) || 0.1;

      const force = SPRING_STIFFNESS * (dist - REST_LENGTH);
      const fx = (dx / dist) * force;
      const fy = (dy / dist) * force;

      nodeA.vx += fx;
      nodeA.vy += fy;
      nodeB.vx -= fx;
      nodeB.vy -= fy;
    }
  }

  // 4. Gravity / Center Force and Viewport Boundaries
  const cx = width / 2;
  const cy = height / 2;
  for (const node of nodes) {
    node.vx += (cx - node.x) * CENTER_GRAVITY;
    node.vy += (cy - node.y) * CENTER_GRAVITY;

    // Apply velocities with damping friction
    node.x += node.vx;
    node.y += node.vy;
    node.vx *= DAMPING;
    node.vy *= DAMPING;

    // Clamp inside viewport boundary
    node.x = Math.max(20, Math.min(width - 20, node.x));
    node.y = Math.max(20, Math.min(height - 20, node.y));
  }

  return { nodes, edges };
}
