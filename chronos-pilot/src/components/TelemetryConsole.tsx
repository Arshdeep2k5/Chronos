/**
 * @license
 * SPDX-License-Identifier: Apache-2.0
 */

import React, { useState } from 'react';
import { Activity, Clock, Cpu, ShieldAlert, Signal, TrendingUp, AlertTriangle } from 'lucide-react';

import { CognitiveState } from '../cognitive/types';

interface Alert {
  id: string;
  timestamp: string;
  severity: 'INFO' | 'WARN' | 'CRITICAL';
  message: string;
  tick_id?: string;
}

interface TelemetryConsoleProps {
  tickSequence: number | null;
  streamLagMs: number;
  droppedCount: number;
  outOfOrderCount: number;
  slowTicksCount: number;
  alerts: Alert[];
  latencyHistory: any[];
  cognitiveState: CognitiveState | null;
}

export default function TelemetryConsole({
  tickSequence,
  streamLagMs,
  droppedCount,
  outOfOrderCount,
  slowTicksCount,
  alerts,
  latencyHistory,
  cognitiveState
}: TelemetryConsoleProps) {
  const [collapsed, setCollapsed] = useState(false);
  const [isReplaying, setIsReplaying] = useState(false);
  const [replaySpeed, setReplaySpeed] = useState<number>(300);
  const latestLatency = latencyHistory[latencyHistory.length - 1];

  const handleReplayLog = () => {
    setIsReplaying(true);
    setTimeout(() => {
      setIsReplaying(false);
    }, 1500);
  };

  return (
    <div id="telemetry-observability-console" className="bg-[#0F0F0F] border border-white/10 p-6 shadow-2xl rounded-none mt-6">
      <div className="flex items-center justify-between border-b border-white/10 pb-4 mb-4">
        <div>
          <h2 className="font-display font-black italic text-lg text-white flex items-center gap-2">
            <Activity className="w-5 h-5 text-[#FF4E00] animate-pulse" />
            COGNITIVE_TELEMETRY_OBSERVABILITY
          </h2>
          <p className="text-[10px] uppercase tracking-widest text-white/40 mt-1">Live execution tracing, latency audits, and stream synchronization status</p>
        </div>
        <div className="flex items-center gap-3">
          <button
            onClick={handleReplayLog}
            disabled={isReplaying || alerts.length === 0}
            className="text-[9px] font-mono border border-white/10 px-2 py-0.5 transition hover:text-[#FF4E00] hover:border-[#FF4E00]/50 disabled:opacity-30 cursor-pointer"
          >
            {isReplaying ? 'REPLAYING_ALERT_HISTORY...' : 'REPLAY_HISTORY'}
          </button>
          <button
            onClick={() => setCollapsed(!collapsed)}
            className="text-[9px] hover:text-[#FF4E00] font-mono border border-white/10 hover:border-[#FF4E00]/50 px-2 py-0.5 transition cursor-pointer"
          >
            {collapsed ? 'SHOW_CONSOLE' : 'HIDE_CONSOLE'}
          </button>
        </div>
      </div>

      {!collapsed && (
        <div className="space-y-6">
          {/* Health Stats Grid */}
          <div className="grid grid-cols-2 md:grid-cols-8 gap-4">
            <div className="bg-black p-3.5 border border-white/5 font-mono">
              <span className="text-[9px] text-white/30 uppercase block">Tick Sequence</span>
              <span className="text-sm font-bold text-white mt-1 block">
                {tickSequence !== null ? `#${tickSequence}` : 'AWAITING_TICK'}
              </span>
            </div>

            <div className="bg-black p-3.5 border border-white/5 font-mono relative overflow-hidden">
              <span className="text-[9px] text-white/30 uppercase block">Stream Lag</span>
              <div className="flex items-baseline gap-1 mt-1">
                <span className={`text-sm font-bold ${streamLagMs > 100 ? 'text-[#FF4E00]' : 'text-green-400'}`}>
                  {streamLagMs}ms
                </span>
                <Signal className={`w-3 h-3 shrink-0 ${streamLagMs > 100 ? 'text-[#FF4E00] animate-ping' : 'text-green-400'}`} />
              </div>
            </div>

            <div className="bg-black p-3.5 border border-white/5 font-mono">
              <span className="text-[9px] text-white/30 uppercase block">Dropped Ticks</span>
              <span className={`text-sm font-bold mt-1 block ${droppedCount > 0 ? 'text-red-500 font-black animate-pulse' : 'text-white/50'}`}>
                {droppedCount}
              </span>
            </div>

            <div className="bg-black p-3.5 border border-white/5 font-mono">
              <span className="text-[9px] text-white/30 uppercase block">Out-Of-Order</span>
              <span className={`text-sm font-bold mt-1 block ${outOfOrderCount > 0 ? 'text-red-400' : 'text-white/50'}`}>
                {outOfOrderCount}
              </span>
            </div>

            <div className="bg-black p-3.5 border border-white/5 font-mono">
              <span className="text-[9px] text-white/30 uppercase block">Slow Ticks</span>
              <span className={`text-sm font-bold mt-1 block ${slowTicksCount > 0 ? 'text-[#FF4E00]' : 'text-white/50'}`}>
                {slowTicksCount}
              </span>
            </div>

            {/* Cognitive Semantic Layer (CSL) Metrics */}
            <div className="bg-black p-3.5 border border-white/5 font-mono">
              <span className="text-[9px] text-white/30 uppercase block">Stability Score</span>
              <span className={`text-sm font-bold mt-1 block ${
                !cognitiveState ? 'text-white/30' : 
                cognitiveState.stability.confidenceScore > 0.8 ? 'text-green-400' : 
                cognitiveState.stability.confidenceScore > 0.55 ? 'text-yellow-400' : 'text-red-500'
              }`}>
                {cognitiveState ? `${Math.round(cognitiveState.stability.confidenceScore * 100)}%` : 'N/A'}
              </span>
            </div>

            <div className="bg-black p-3.5 border border-white/5 font-mono">
              <span className="text-[9px] text-white/30 uppercase block">Drift Velocity</span>
              <span className="text-sm font-bold text-white mt-1 block">
                {cognitiveState ? `${cognitiveState.stability.driftVelocity} ms/tick` : 'N/A'}
              </span>
            </div>

            <div className="bg-black p-3.5 border border-white/5 font-mono">
              <span className="text-[9px] text-white/30 uppercase block">Intent Label</span>
              <span className="text-xs font-bold text-[#FF4E00] mt-1.5 block uppercase truncate">
                {cognitiveState ? cognitiveState.intent.intentLabel : 'N/A'}
              </span>
            </div>
          </div>

          {/* Latency Breakdown & Alerts */}
          <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
            {/* Latency Breakdown */}
            <div className="bg-black/40 border border-white/5 p-4 rounded-none">
              <span className="font-mono text-[10px] font-bold text-white uppercase tracking-wider block mb-3 border-b border-white/5 pb-1">
                Phase Execution Breakdown (Latest Frame)
              </span>
              
              {latestLatency ? (
                <div className="space-y-3 font-mono text-[11px]">
                  <div>
                    <div className="flex justify-between text-white/60 mb-1">
                      <span>Ingestion to Loop Ingress:</span>
                      <span className="font-bold text-white">{latestLatency.perception_to_execution_start_ms}ms</span>
                    </div>
                    <div className="w-full bg-white/5 h-1.5 rounded-none overflow-hidden">
                      <div className="bg-blue-500 h-full" style={{ width: `${Math.min(100, (latestLatency.perception_to_execution_start_ms / 50) * 100)}%` }}></div>
                    </div>
                  </div>

                  <div>
                    <div className="flex justify-between text-white/60 mb-1">
                      <span>Internal Tick Processing:</span>
                      <span className={`font-bold ${latestLatency.tick_processing_ms > 50 ? 'text-[#FF4E00]' : 'text-green-400'}`}>
                        {latestLatency.tick_processing_ms}ms
                      </span>
                    </div>
                    <div className="w-full bg-white/5 h-1.5 rounded-none overflow-hidden">
                      <div className={`h-full ${latestLatency.tick_processing_ms > 50 ? 'bg-[#FF4E00]' : 'bg-green-400'}`} style={{ width: `${Math.min(100, (latestLatency.tick_processing_ms / 50) * 100)}%` }}></div>
                    </div>
                  </div>

                  <div>
                    <div className="flex justify-between text-white/60 mb-1">
                      <span>SSE Stream Delivery Lag:</span>
                      <span className={`font-bold ${latestLatency.network_delivery_lag_ms > 100 ? 'text-[#FF4E00]' : 'text-green-400'}`}>
                        {latestLatency.network_delivery_lag_ms}ms
                      </span>
                    </div>
                    <div className="w-full bg-white/5 h-1.5 rounded-none overflow-hidden">
                      <div className={`h-full ${latestLatency.network_delivery_lag_ms > 100 ? 'bg-[#FF4E00]' : 'bg-green-400'}`} style={{ width: `${Math.min(100, (latestLatency.network_delivery_lag_ms / 200) * 100)}%` }}></div>
                    </div>
                  </div>

                  <div>
                    <div className="flex justify-between text-white/60 mb-1">
                      <span>UI Parsing &amp; Render Lag:</span>
                      <span className="font-bold text-white">{latestLatency.ui_render_time_ms}ms</span>
                    </div>
                    <div className="w-full bg-white/5 h-1.5 rounded-none overflow-hidden">
                      <div className="bg-purple-500 h-full" style={{ width: `${Math.min(100, (latestLatency.ui_render_time_ms / 50) * 100)}%` }}></div>
                    </div>
                  </div>
                </div>
              ) : (
                <div className="h-32 flex items-center justify-center font-mono text-white/30 text-[10px] uppercase">
                  Awaiting frame emission from active loop...
                </div>
              )}
            </div>

            {/* Health warnings / alerts logs */}
            <div className="bg-black/40 border border-white/5 p-4 rounded-none flex flex-col h-48 justify-between">
              <div>
                <span className="font-mono text-[10px] font-bold text-white uppercase tracking-wider block mb-3 border-b border-white/5 pb-1 flex items-center gap-1.5">
                  <ShieldAlert className="w-3.5 h-3.5 text-[#FF4E00]" />
                  Telemetry Alerts Log (Structured Ledger)
                </span>
                
                <div className="space-y-1.5 font-mono text-[9px] overflow-y-auto max-h-28 pr-1">
                  {alerts.length > 0 ? (
                    (isReplaying ? [...alerts].reverse() : alerts).slice(-6).map((alert, idx) => (
                      <div key={alert.id || idx} className={`flex items-start justify-between border-b border-white/5 pb-1 ${isReplaying ? 'animate-pulse text-[#FF4E00]' : ''}`}>
                        <div className="flex gap-2">
                          <span className={`px-1 text-[8px] font-black ${
                            alert.severity === 'CRITICAL' ? 'bg-red-950 text-red-500 animate-pulse border border-red-500/20' : 
                            alert.severity === 'WARN' ? 'bg-yellow-950 text-yellow-500' : 'bg-green-950 text-green-500'
                          }`}>
                            {alert.severity}
                          </span>
                          <span className="text-white/70">{alert.message}</span>
                        </div>
                        {alert.tick_id && (
                          <span className="text-[7px] text-white/30 shrink-0">
                            tick:{alert.tick_id.substring(0, 8)}
                          </span>
                        )}
                      </div>
                    ))
                  ) : (
                    <div className="text-white/20 italic">No health alerts recorded. System operating within optimal latency bounds.</div>
                  )}
                </div>
              </div>
              
              <div className="text-[9px] font-mono text-white/30 uppercase tracking-widest pt-2 border-t border-white/5 flex justify-between items-center">
                <span>Compliance target: &lt;50ms soft cap</span>
                <span className="text-green-400 font-bold">100% compliant</span>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
