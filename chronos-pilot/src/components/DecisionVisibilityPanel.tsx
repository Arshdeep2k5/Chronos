/**
 * @license
 * SPDX-License-Identifier: Apache-2.0
 */

import React from 'react';
import { Eye, ShieldAlert, Cpu, Activity, Info } from 'lucide-react';
import { CognitiveState } from '../cognitive/types';
import CognitiveGraphView from '../cognitive-visualization/CognitiveGraphView';

interface DecisionVisibilityPanelProps {
  cognitiveState: CognitiveState | null;
}

export default function DecisionVisibilityPanel({ cognitiveState }: DecisionVisibilityPanelProps) {
  if (!cognitiveState) {
    return (
      <div className="bg-[#0F0F0F] border border-white/10 p-6 rounded-none text-center font-mono text-white/30 text-xs uppercase">
        Awaiting cognitive state reconstruction...
      </div>
    );
  }

  const { intent, load, pressure, stability, graph } = cognitiveState;

  // Determine health colors
  const stabilityColor = stability.confidenceScore > 0.8 ? 'text-green-400' : stability.confidenceScore > 0.55 ? 'text-yellow-400' : 'text-red-500';
  const loadColor = load.loadLevel === 'LOW' ? 'text-green-400' : load.loadLevel === 'MEDIUM' ? 'text-blue-400' : load.loadLevel === 'HIGH' ? 'text-yellow-400' : 'text-red-500';
  const pressureColor = pressure.signalLevel === 'NORMAL' ? 'text-green-400' : pressure.signalLevel === 'WARN' ? 'text-yellow-400' : 'text-red-500';

  return (
    <div id="decision-visibility-panel" className="bg-[#0F0F0F] border border-white/10 p-6 shadow-2xl rounded-none mt-6">
      <div className="flex items-center justify-between border-b border-white/10 pb-4 mb-4">
        <div>
          <h2 className="font-display font-black italic text-lg text-white flex items-center gap-2">
            <Eye className="w-5 h-5 text-[#FF4E00]" />
            COGNITIVE_SEMANTIC_LAYER_INTERPRETATION
          </h2>
          <p className="text-[10px] uppercase tracking-widest text-white/40 mt-1">Real-time self-interpreting execution state and logical intent analysis</p>
        </div>
        <div className="flex items-center gap-2">
          <span className="text-[9px] font-mono border border-white/10 px-2 py-0.5 bg-white/5 text-white/60">
            CSL_V1.0
          </span>
        </div>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Left Column: Natural Language Explanation & Interactive Topology View */}
        <div className="lg:col-span-2 space-y-4">
          <div className="bg-black/50 border border-white/5 p-5 relative overflow-hidden">
            <span className="absolute top-0 right-0 w-24 h-24 bg-[#FF4E00]/5 rounded-full blur-xl pointer-events-none"></span>
            
            <div className="flex items-center gap-2 mb-3">
              <Info className="w-4 h-4 text-[#FF4E00]" />
              <span className="font-mono text-[10px] font-bold text-white uppercase tracking-wider">Natural Language Execution State</span>
            </div>
            
            <div className="font-mono text-sm text-white/90 leading-relaxed border-l-2 border-[#FF4E00] pl-4 my-4">
              "System is currently operating in <strong className="text-white italic">{intent.intentLabel}</strong> mode. {intent.description} 
              With a stability score of <strong className={stabilityColor}>{Math.round(stability.confidenceScore * 100)}%</strong> and 
              drift velocity measured at <strong className="text-white">{stability.driftVelocity} ms/tick</strong>, the system shows 
              a <strong className="text-white">{stability.instabilitySignal}</strong> operational signal. Cognitive load levels are evaluated 
              as <strong className={loadColor}>{load.loadLevel}</strong> due to an event processing rate of {load.eventProcessingRate} events/sec 
              and backlog size of {pressure.backlogSize} commitments."
            </div>
          </div>

          {/* Interactive Topology Graph */}
          <CognitiveGraphView
            intentGraph={graph}
            stabilityScore={stability.confidenceScore}
            backlogSize={pressure.backlogSize}
          />
        </div>

        {/* Right Column: Gauges & Metrics */}
        <div className="space-y-4">
          <div className="bg-black p-4 border border-white/5 font-mono">
            <span className="text-[9px] text-white/30 uppercase block mb-1">Intent Intensity</span>
            <div className="flex justify-between items-baseline mb-2">
              <span className="text-sm font-bold text-white">{intent.intentLabel}</span>
              <span className="text-xs text-white/60">{Math.round(intent.intensity * 100)}%</span>
            </div>
            <div className="w-full bg-white/5 h-2 rounded-none overflow-hidden">
              <div className="bg-[#FF4E00] h-full" style={{ width: `${intent.intensity * 100}%` }}></div>
            </div>
          </div>

          <div className="bg-black p-4 border border-white/5 font-mono">
            <span className="text-[9px] text-white/30 uppercase block mb-1">Reasoning Complexity</span>
            <div className="flex justify-between items-baseline mb-2">
              <span className={`text-sm font-bold ${loadColor}`}>{load.loadLevel}</span>
              <span className="text-xs text-white/60">{Math.round(load.reasoningComplexityScore * 100)}%</span>
            </div>
            <div className="w-full bg-white/5 h-2 rounded-none overflow-hidden">
              <div className="bg-blue-500 h-full" style={{ width: `${load.reasoningComplexityScore * 100}%` }}></div>
            </div>
          </div>

          <div className="bg-black p-4 border border-white/5 font-mono">
            <span className="text-[9px] text-white/30 uppercase block mb-1">Execution Pressure</span>
            <div className="flex justify-between items-baseline mb-2">
              <span className={`text-sm font-bold ${pressureColor}`}>{pressure.signalLevel}</span>
              <span className="text-xs text-white/60">backlog: {pressure.backlogSize}</span>
            </div>
            <div className="w-full bg-white/5 h-2 rounded-none overflow-hidden">
              <div className={`h-full ${pressure.signalLevel === 'CRITICAL' ? 'bg-red-500' : 'bg-green-400'}`} style={{ width: `${Math.min(100, (pressure.backlogSize / 10) * 100)}%` }}></div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
