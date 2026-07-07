/**
 * @license
 * SPDX-License-Identifier: Apache-2.0
 */

import React from 'react';
import { AlertCircle, TrendingUp, Calendar, Info, Clock, ArrowRight } from 'lucide-react';
import { Commitment } from '../types';

interface RiskForecasterProps {
  commitments: Commitment[];
}

export default function RiskForecaster({ commitments }: RiskForecasterProps) {
  // Highlight the highest-risk commitment for comprehensive trajectory display
  const highRiskCommitment = commitments.find(c => c.health === 'RED') || commitments[0];

  if (!highRiskCommitment) {
    return (
      <div className="bg-[#111827] border border-[#1f2937] rounded-xl p-5 text-center text-slate-400">
        No active commitments discovered yet. Use the Sandbox to download a deliverables PDF.
      </div>
    );
  }

  // Extract variables computed on backend
  const riskScore = highRiskCommitment.risk_score || 0;
  const completionChance = highRiskCommitment.completion_chance || 0;
  const simulatedFailureDate = highRiskCommitment.risk_score && highRiskCommitment.risk_score > 1
    ? (highRiskCommitment as any).simulated_failure_date
    : 'Before Deadline';
  const marginalLoss = (highRiskCommitment as any).marignal_loss_24h || 0;

  // Let's generate coordinates for an SVG line chart mapping risk decay trajectory
  // The line represents completion probability P_comp falling as time travels / task delay occurs
  const points: { x: number; y: number; label: string }[] = [];
  const totalDaysSimulated = 7;
  for (let i = 0; i <= totalDaysSimulated; i++) {
    const x = (i / totalDaysSimulated) * 100; // SVG space percentage width
    
    // Compute P_comp at delay interval using the logistic decay simulation
    const simulatedRisk = riskScore * (1 + (i * 0.15)); // risk grows as days pass without progress
    const simulatedChance = 1 / (1 + Math.exp(6 * (simulatedRisk - 1.0)));
    const y = 100 - (simulatedChance * 100); // SVG height matches 0-100 (inverting since SVG y=0 is top)
    points.push({ x, y, label: `Day +${i}` });
  }

  const svgPath = points.map((p, i) => `${i === 0 ? 'M' : 'L'} ${p.x}% ${p.y}%`).join(' ');

  return (
    <div id="risk-panel" className="bg-[#0F0F0F] border border-white/10 p-6 shadow-2xl rounded-none">
      <div className="flex items-center justify-between border-b border-white/10 pb-4 mb-4">
        <div>
          <h2 className="font-display font-black italic text-lg text-white flex items-center gap-2">
            <AlertCircle className="w-5 h-5 text-[#FF4E00]" />
            FAILURE_FORECASTING
          </h2>
          <p className="text-[10px] uppercase tracking-widest text-white/40 mt-1">Predictive timeline decay & velocity diagnostics</p>
        </div>
        <span className="text-[9px] uppercase tracking-[0.2em] bg-white/5 border border-white/10 text-white/60 font-semibold px-2 py-0.5 rounded-none font-mono">
          MODEL_MC_V1.0
        </span>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-3 gap-4 mb-5">
        {/* Core Stats */}
        <div className="bg-[#121212] border border-white/5 rounded-none p-4 flex flex-col justify-between">
          <span className="text-[10px] uppercase tracking-widest text-white/40 font-mono">RISK_INDEX</span>
          <div className="my-2">
            <span className={`text-3xl font-display font-bold italic ${riskScore >= 1.0 ? 'text-[#FF4E00] animate-pulse' : 'text-white'}`}>
              {riskScore.toFixed(2)}
            </span>
            <span className="text-[9px] text-[#FF4E00] font-mono ml-1.5 font-bold">({riskScore >= 1.0 ? 'CRIT' : 'WARN'})</span>
          </div>
          <span className="text-[9px] uppercase text-white/30 font-mono">Effort / Capacity</span>
        </div>

        <div className="bg-[#121212] border border-white/5 rounded-none p-4 flex flex-col justify-between">
          <span className="text-[10px] uppercase tracking-widest text-white/40 font-mono">PROBABILITY</span>
          <div className="my-2">
            <span className={`text-3xl font-display font-bold italic ${completionChance <= 0.40 ? 'text-[#FF4E00]' : 'text-white'}`}>
              {(completionChance * 100).toFixed(0)}%
            </span>
          </div>
          <span className="text-[9px] uppercase text-white/30 font-mono">Monte Carlo paths</span>
        </div>

        <div className="bg-[#121212] border border-white/5 rounded-none p-4 flex flex-col justify-between">
          <span className="text-[10px] uppercase tracking-widest text-white/40 font-mono">EST_FAILURE</span>
          <div className="my-2">
            <span className="text-xs font-bold font-mono text-[#FF4E00] flex items-center gap-1">
              <Calendar className="w-3.5 h-3.5 text-[#FF4E00]" />
              {simulatedFailureDate}
            </span>
          </div>
          <span className="text-[9px] uppercase text-white/30 font-mono">Capacity breach</span>
        </div>
      </div>

      {/* Trajectory visualization SVG */}
      <div className="bg-black border border-white/10 rounded-none p-4 mb-5">
        <div className="flex items-center justify-between mb-3">
          <div className="flex items-center gap-1.5">
            <TrendingUp className="w-4 h-4 text-[#FF4E00]" />
            <span className="text-[10px] uppercase tracking-widest text-white">DECAY_TRAJECTORY_CURVE</span>
          </div>
          <span className="text-[9px] font-mono text-white/30">X: DELAY | Y: PROB</span>
        </div>

        {/* SVG Line Chart */}
        <div className="relative h-28 w-full border-b border-l border-white/10 pb-2 pl-2">
          {/* Grid lines */}
          <div className="absolute inset-0 grid grid-rows-3 opacity-10 pointer-events-none">
            <div className="border-t border-white"></div>
            <div className="border-t border-white"></div>
            <div className="border-t border-white"></div>
          </div>

          <svg className="h-full w-full overflow-visible" preserveAspectRatio="none">
            <path
              d={svgPath}
              fill="none"
              stroke="#FF4E00"
              strokeWidth="2.5"
              className="transition-all duration-300"
            />
            {/* Draw dots */}
            {points.map((p, idx) => (
              <circle
                key={idx}
                cx={`${p.x}%`}
                cy={`${p.y}%`}
                r="3.5"
                fill={idx === 0 ? '#FFFFFF' : '#FF4E00'}
                className="hover:scale-150 transition-transform cursor-pointer"
              />
            ))}
          </svg>

          {/* Probability labels */}
          <div className="absolute left-1 top-1 text-[8px] font-mono text-white/30">100% SUCCESS</div>
          <div className="absolute left-1 bottom-3 text-[8px] font-mono text-white/30">0% FAILURE</div>
        </div>

        {/* X-axis labels */}
        <div className="flex justify-between text-[8px] font-mono text-white/40 mt-1.5 pl-4">
          <span>TODAY</span>
          <span>+2 DAYS</span>
          <span>+4 DAYS</span>
          <span>+7 DAYS</span>
        </div>
      </div>

      {/* Consequence Simulator */}
      <div className="bg-[#121212] border border-[#FF4E00]/20 rounded-none p-4 flex gap-3">
        <Info className="w-5 h-5 text-[#FF4E00] shrink-0 mt-0.5" />
        <div className="flex-1">
          <h4 className="text-[10px] uppercase tracking-[0.4em] text-[#FF4E00] font-bold">
            CONSEQUENCE_FEED
          </h4>
          <p className="text-xs text-white/60 mt-2 leading-relaxed">
            If you postpone work on <span className="font-semibold text-white">"{highRiskCommitment.title}"</span> by just 24 hours:
          </p>
          <div className="flex flex-col sm:flex-row sm:items-center gap-2 mt-3">
            <div className="bg-black px-3 py-1.5 border border-white/5 text-[11px] font-mono text-white/70">
              Prob: <span className="text-white">{(completionChance * 100).toFixed(0)}%</span>
            </div>
            <ArrowRight className="w-3.5 h-3.5 text-[#FF4E00] self-center hidden sm:block" />
            <div className="bg-[#FF4E00]/10 px-3 py-1.5 border border-[#FF4E00] text-[11px] font-mono text-[#FF4E00] font-black">
              Prob: {((completionChance - marginalLoss) * 100).toFixed(0)}%
            </div>
          </div>
          <p className="text-[10px] text-[#FF4E00] font-bold uppercase tracking-wider mt-3">
            ⚠️ DELAY BREACHES THRESHOLD BY {(riskScore * 10).toFixed(1)} HOURS!
          </p>
        </div>
      </div>
    </div>
  );
}
