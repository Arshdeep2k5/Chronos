/**
 * @license
 * SPDX-License-Identifier: Apache-2.0
 */

import React, { useEffect, useState, useRef } from 'react';
import { IntentGraph } from '../cognitive/types';
import { GraphState, VisualNode } from './graphTypes';
import { transformGraph } from './graphEngine';
import { initializeNodePositions, stepSimulation } from './layoutEngine';

interface CognitiveGraphViewProps {
  intentGraph: IntentGraph;
  stabilityScore: number;
  backlogSize: number;
}

export default function CognitiveGraphView({
  intentGraph,
  stabilityScore,
  backlogSize
}: CognitiveGraphViewProps) {
  const width = 600;
  const height = 300;
  const [graph, setGraph] = useState<GraphState>({ nodes: [], edges: [] });
  const [hoveredNode, setHoveredNode] = useState<VisualNode | null>(null);
  const [draggedNodeId, setDraggedNodeId] = useState<string | null>(null);
  
  const simulationRef = useRef<number | null>(null);
  const graphStateRef = useRef<GraphState>({ nodes: [], edges: [] });

  // 1. Sync new incoming nodes while preserving existing coordinates
  useEffect(() => {
    const freshVisualState = transformGraph(intentGraph, stabilityScore, backlogSize);
    
    // Map new nodes preserving coords of match IDs
    const mergedNodes = freshVisualState.nodes.map(newNode => {
      const existing = graphStateRef.current.nodes.find(n => n.id === newNode.id);
      if (existing) {
        return {
          ...newNode,
          x: existing.x,
          y: existing.y,
          vx: existing.vx,
          vy: existing.vy
        };
      }
      return newNode;
    });

    const initialized = initializeNodePositions(mergedNodes, width, height);
    const nextState = {
      nodes: initialized,
      edges: freshVisualState.edges
    };

    graphStateRef.current = nextState;
    setGraph(nextState);
  }, [intentGraph, stabilityScore, backlogSize]);

  // 2. Start RequestAnimationFrame Simulation Loop
  useEffect(() => {
    const loop = () => {
      if (draggedNodeId) {
        // Skip physics steps for dragged node to prevent spring jitter
      }
      const nextState = stepSimulation(graphStateRef.current, width, height);
      graphStateRef.current = nextState;
      setGraph(nextState);
      simulationRef.current = requestAnimationFrame(loop);
    };

    simulationRef.current = requestAnimationFrame(loop);
    return () => {
      if (simulationRef.current) {
        cancelAnimationFrame(simulationRef.current);
      }
    };
  }, [draggedNodeId]);

  // 3. Drag Handlers
  const handleMouseDown = (nodeId: string) => {
    setDraggedNodeId(nodeId);
  };

  const handleMouseMove = (e: React.MouseEvent<SVGSVGElement>) => {
    if (!draggedNodeId) return;

    const rect = e.currentTarget.getBoundingClientRect();
    const x = ((e.clientX - rect.left) / rect.width) * width;
    const y = ((e.clientY - rect.top) / rect.height) * height;

    const updatedNodes = graphStateRef.current.nodes.map(n => {
      if (n.id === draggedNodeId) {
        return { ...n, x, y, vx: 0, vy: 0 };
      }
      return n;
    });

    const nextState = {
      ...graphStateRef.current,
      nodes: updatedNodes
    };
    graphStateRef.current = nextState;
    setGraph(nextState);
  };

  const handleMouseUpOrLeave = () => {
    setDraggedNodeId(null);
  };

  return (
    <div className="relative border border-white/5 bg-black/40 p-4 select-none">
      <div className="flex justify-between items-center mb-3">
        <span className="font-mono text-[9px] uppercase tracking-wider text-white/50 block">
          Interactive Cognitive Field Topology View
        </span>
        {hoveredNode && (
          <span className="font-mono text-[9px] text-[#FF4E00] animate-pulse">
            inspecting: {hoveredNode.label}
          </span>
        )}
      </div>

      <svg
        viewBox={`0 0 ${width} ${height}`}
        className="w-full h-auto cursor-grab active:cursor-grabbing border border-white/5 bg-black/20"
        onMouseMove={handleMouseMove}
        onMouseUp={handleMouseUpOrLeave}
        onMouseLeave={handleMouseUpOrLeave}
      >
        <defs>
          <style>{`
            @keyframes svg-pulse {
              0% { r: 6px; opacity: 0.8; stroke-width: 1px; }
              100% { r: 24px; opacity: 0; stroke-width: 2px; }
            }
            .pulse-ring {
              animation: svg-pulse 1.8s cubic-bezier(0.215, 0.61, 0.355, 1) infinite;
            }
          `}</style>
        </defs>

        {/* 1. Render Edges */}
        {graph.edges.map((edge) => {
          const sourceNode = graph.nodes.find((n) => n.id === edge.source);
          const targetNode = graph.nodes.find((n) => n.id === edge.target);

          if (!sourceNode || !targetNode) return null;

          return (
            <line
              key={edge.id}
              x1={sourceNode.x}
              y1={sourceNode.y}
              x2={targetNode.x}
              y2={targetNode.y}
              stroke={edge.color}
              strokeWidth={edge.style === 'solid' ? 2 : edge.style === 'dashed' ? 1.5 : 1}
              strokeDasharray={edge.style === 'dashed' ? '4 4' : undefined}
              opacity={0.65}
            />
          );
        })}

        {/* 2. Render Nodes */}
        {graph.nodes.map((node) => (
          <g
            key={node.id}
            transform={`translate(${node.x}, ${node.y})`}
            onMouseEnter={() => setHoveredNode(node)}
            onMouseLeave={() => setHoveredNode(null)}
            onMouseDown={() => handleMouseDown(node.id)}
            className="cursor-pointer"
          >
            {/* Pulsing ring for warnings and hotspots */}
            {node.pulse && (
              <circle
                r={node.size}
                fill="none"
                stroke={node.color}
                className="pulse-ring"
              />
            )}

            {/* Core Node Circle */}
            <circle
              r={node.size / 2}
              fill={node.color}
              stroke="#000"
              strokeWidth={1.5}
            />

            {/* Small Label Text */}
            <text
              y={-node.size - 2}
              textAnchor="middle"
              fill="#FFFFFF"
              fontSize="7px"
              fontFamily="monospace"
              opacity={hoveredNode?.id === node.id ? 1 : 0.65}
              className="pointer-events-none"
            >
              {node.label}
            </text>
          </g>
        ))}
      </svg>

      {/* Hover Info Tooltip overlay */}
      {hoveredNode && (
        <div className="absolute bottom-6 left-6 bg-[#0B0B0B] border border-white/10 p-3 font-mono text-[9px] text-white/80 shadow-2xl max-w-xs">
          <span className="text-[#FF4E00] font-black uppercase tracking-wider block border-b border-white/5 pb-1 mb-1">
            NODE_METADATA_INSPECT
          </span>
          <div>Type: <strong className="text-white">{hoveredNode.type}</strong></div>
          <div>Label: <strong className="text-white">{hoveredNode.label}</strong></div>
          <div>Timestamp: <strong className="text-white">{new Date(hoveredNode.metadata?.timestamp).toLocaleTimeString()}</strong></div>
          <div className="text-white/40 mt-1.5 leading-normal">{hoveredNode.metadata?.derivedInfo}</div>
        </div>
      )}
    </div>
  );
}
