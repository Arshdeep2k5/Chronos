/**
 * @license
 * SPDX-License-Identifier: Apache-2.0
 */

import React, { useState } from 'react';
import { AlertCircle, TrendingUp, Calendar, Info, Clock, ArrowRight } from 'lucide-react';
import { Commitment } from '../types';
import { API_BASE } from '../config';

interface RiskForecasterProps {
  commitments: Commitment[];
}

export default function RiskForecaster({ commitments }: RiskForecasterProps) {
  const [isManualModalOpen, setIsManualModalOpen] = useState(false);
  const [ingestType, setIngestType] = useState('DEADLINE');
  const [ingestTitle, setIngestTitle] = useState('');
  const [ingestDetail, setIngestDetail] = useState('');
  const [isSubmitting, setIsSubmitting] = useState(false);

  const handleIngest = async () => {
    setIsSubmitting(true);
    try {
      const payload = {
        displayName: ingestTitle,
        detail: ingestDetail,
        entityType: ingestType === 'DEADLINE' ? 'TEXT' : ingestType // Map to supported types
      };
      
      const res = await fetch(`${API_BASE}/api/telemetry/ingest`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(payload)
      });
      
      if (res.ok) {
        setIngestTitle('');
        setIngestDetail('');
        setIsManualModalOpen(false);
      }
    } catch (e) {
      console.error(e);
    } finally {
      setIsSubmitting(false);
    }
  };

  const highRiskCommitment = commitments.find(c => c.health === 'RED') || commitments[0];

  const renderContent = () => {
    if (!highRiskCommitment) {
      return (
        <div className="bg-[#111827] border border-[#1f2937] p-5 text-center text-slate-400 mb-5">
          No active commitments discovered yet. Use Manual Ingestion to add one.
        </div>
      );
    }

    const riskScore = highRiskCommitment.risk_score || 0;
    const completionChance = highRiskCommitment.completion_chance || 0;
    const simulatedFailureDate = highRiskCommitment.risk_score && highRiskCommitment.risk_score > 1
      ? (highRiskCommitment as any).simulated_failure_date
      : 'Before Deadline';
    const marginalLoss = (highRiskCommitment as any).marignal_loss_24h || 0;

    const points: { x: number; y: number; label: string }[] = [];
    const totalDaysSimulated = 7;
    for (let i = 0; i <= totalDaysSimulated; i++) {
      const x = (i / totalDaysSimulated) * 100;
      const simulatedRisk = riskScore * (1 + (i * 0.15));
      const simulatedChance = 1 / (1 + Math.exp(6 * (simulatedRisk - 1.0)));
      const y = 100 - (simulatedChance * 100);
      points.push({ x, y, label: `Day +${i}` });
    }

    const svgPath = points.map((p, i) => `${i === 0 ? 'M' : 'L'} ${p.x}% ${p.y}%`).join(' ');

    return (
      <>
        <div className="grid grid-cols-1 md:grid-cols-3 gap-4 mb-5">
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

        <div className="bg-black border border-white/10 rounded-none p-4 mb-5">
          <div className="flex items-center justify-between mb-3">
            <div className="flex items-center gap-1.5">
              <TrendingUp className="w-4 h-4 text-[#FF4E00]" />
              <span className="text-[10px] uppercase tracking-widest text-white">DECAY_TRAJECTORY_CURVE</span>
            </div>
            <span className="text-[9px] font-mono text-white/30">X: DELAY | Y: PROB</span>
          </div>

          <div className="relative h-28 w-full border-b border-l border-white/10 pb-2 pl-2">
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

            <div className="absolute left-1 top-1 text-[8px] font-mono text-white/30">100% SUCCESS</div>
            <div className="absolute left-1 bottom-3 text-[8px] font-mono text-white/30">0% FAILURE</div>
          </div>

          <div className="flex justify-between text-[8px] font-mono text-white/40 mt-1.5 pl-4">
            <span>TODAY</span>
            <span>+2 DAYS</span>
            <span>+4 DAYS</span>
            <span>+7 DAYS</span>
          </div>
        </div>

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
      </>
    );
  };

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

      {renderContent()}

      {/* Manual Ingestion Button */}
      <div className="mt-5 border-t border-white/10 pt-5 text-center">
        <button 
          onClick={() => setIsManualModalOpen(true)}
          className="bg-[#0F0F0F] border border-[#FF4E00]/50 hover:bg-[#FF4E00]/10 hover:border-[#FF4E00] text-[#FF4E00] font-mono text-[10px] font-bold uppercase tracking-widest px-6 py-2 transition-all cursor-pointer inline-flex items-center gap-2"
        >
          <TrendingUp className="w-3.5 h-3.5" />
          MANUAL INGESTION
        </button>
      </div>

      {/* Manual Ingestion Modal */}
      {isManualModalOpen && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/80 backdrop-blur-sm p-4">
          <div className="bg-[#070707] border border-[#FF4E00] w-full max-w-md p-6 relative shadow-[0_0_20px_rgba(255,78,0,0.2)]">
            
            <button 
              onClick={() => setIsManualModalOpen(false)}
              className="absolute top-4 right-4 text-white/40 hover:text-white"
            >
              ✕
            </button>
            
            <h3 className="font-display font-black italic text-lg text-white mb-1 uppercase">
              Manual Context Ingestion
            </h3>
            <p className="text-[10px] font-mono text-white/50 uppercase tracking-widest mb-6">
              Bypass scraping. Inject sovereign context directly.
            </p>
            
            <div className="space-y-4">
              <div>
                <label className="block text-[9px] font-mono uppercase text-[#FF4E00] font-bold mb-1.5">Context Type</label>
                <select 
                  value={ingestType}
                  onChange={e => setIngestType(e.target.value)}
                  className="w-full bg-[#111] border border-white/20 text-white text-xs font-mono p-2.5 outline-none focus:border-[#FF4E00]"
                >
                  <option value="DEADLINE">Strategic Deadline</option>
                  <option value="FILE">Local File/Document</option>
                  <option value="URL">External Link</option>
                  <option value="IDEA">Raw Idea / Brainstorm</option>
                </select>
              </div>
              
              <div>
                <label className="block text-[9px] font-mono uppercase text-[#FF4E00] font-bold mb-1.5">Display Title</label>
                <input 
                  type="text"
                  value={ingestTitle}
                  onChange={e => setIngestTitle(e.target.value)}
                  placeholder="e.g. Final Presentation Deck v2"
                  className="w-full bg-[#111] border border-white/20 text-white text-xs font-mono p-2.5 outline-none focus:border-[#FF4E00]"
                />
              </div>

              <div>
                <label className="block text-[9px] font-mono uppercase text-[#FF4E00] font-bold mb-1.5">Detail / Content / URL</label>
                <textarea 
                  value={ingestDetail}
                  onChange={e => setIngestDetail(e.target.value)}
                  placeholder="Enter file path, URL, or raw text content..."
                  className="w-full bg-[#111] border border-white/20 text-white text-xs font-mono p-2.5 outline-none focus:border-[#FF4E00] min-h-[80px]"
                />
              </div>
              
              <button 
                onClick={handleIngest}
                disabled={isSubmitting || !ingestTitle || !ingestDetail}
                className="w-full bg-[#FF4E00] text-black font-mono text-xs font-black uppercase tracking-widest py-3 mt-2 hover:bg-white disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
              >
                {isSubmitting ? 'INGESTING...' : 'INJECT CONTEXT TO TIMELINE'}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
