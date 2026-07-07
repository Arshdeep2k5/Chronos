/**
 * @license
 * SPDX-License-Identifier: Apache-2.0
 */

import React, { useState } from 'react';
import { Calendar, HelpCircle, AlertOctagon, CheckSquare, Sparkles, ChevronDown, ChevronUp, Loader2, Hourglass } from 'lucide-react';
import { motion, AnimatePresence } from 'motion/react';
import { Commitment, ProjectAction } from '../types';

interface CommitmentListProps {
  commitments: Commitment[];
  actions: ProjectAction[];
  onToggleAction: (actionId: number) => void;
  onPlanGenerated: (plan: any) => void;
}

export default function CommitmentList({
  commitments,
  actions,
  onToggleAction,
  onPlanGenerated
}: CommitmentListProps) {
  const [expandedActionId, setExpandedActionId] = useState<number | null>(null);
  const [loadingPlanId, setLoadingPlanId] = useState<number | null>(null);
  const [recoveryPlans, setRecoveryPlans] = useState<{ [commitmentId: number]: any[] }>({});

  const handleGeneratePlan = async (commitmentId: number) => {
    setLoadingPlanId(commitmentId);
    try {
      const res = await fetch('/api/execution/generate-recovery-plan', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ project_hint: undefined })
      });
      const data = await res.json();
      if (data.ok) {
        const planData = data.data;
        const payload = planData.recovery_plan?.payload || {};
        const recommended = payload.recommended_next_action || "Resume development on stalled commitments";
        const reentryPoints = payload.dormant_project_reentry_points || [];
        const trajectories = Object.entries(payload.project_recovery_trajectories || {}).map(([k, v]) => `${k}: ${v}`);

        const parsedPlan = [
          { day: '[Today]', task: recommended, hours: 4 },
          ...reentryPoints.map((f: any, idx: number) => ({
            day: idx === 0 ? '[Tomorrow]' : `[Day ${idx + 2}]`,
            task: `Review and resume work on: ${f}`,
            hours: 2
          })),
          ...trajectories.map((t: any, idx: number) => ({
            day: `[Day ${reentryPoints.length + idx + 2}]`,
            task: `Address trajectory: ${t}`,
            hours: 3
          }))
        ];

        setRecoveryPlans(prev => ({
          ...prev,
          [commitmentId]: parsedPlan
        }));
        onPlanGenerated(parsedPlan);
      }
    } catch (e) {
      console.error(e);
    } finally {
      setLoadingPlanId(null);
    }
  };

  const getHealthStyle = (health?: string) => {
    switch (health) {
      case 'GREEN':
        return { bg: 'bg-[#121212] border-white/10', text: 'text-white/70', dot: 'bg-white' };
      case 'YELLOW':
        return { bg: 'bg-[#121212] border-[#FF4E00]/40', text: 'text-[#FF4E00]', dot: 'bg-[#FF4E00]' };
      case 'RED':
        return { bg: 'bg-black border-[#FF4E00]', text: 'text-black bg-[#FF4E00] px-1.5 py-0.5 font-black uppercase tracking-[0.1em]', dot: 'bg-[#FF4E00]' };
      default:
        return { bg: 'bg-[#121212] border-white/5', text: 'text-white/40', dot: 'bg-white/30' };
    }
  };

  const formatDeadline = (dateStr?: string) => {
    if (!dateStr) return 'No Deadline';
    try {
      return new Date(dateStr).toLocaleDateString('en-US', {
        month: 'short',
        day: 'numeric',
        year: 'numeric'
      }).toUpperCase();
    } catch (e) {
      return dateStr;
    }
  };

  return (
    <div id="commitments-panel" className="space-y-6">
      {commitments.map((commitment, index) => {
        const healthStyle = getHealthStyle(commitment.health);
        const projectActions = actions.filter(a => a.project_id === commitment.project_id);
        const activePlan = recoveryPlans[commitment.id];

        return (
          <motion.div
            key={commitment.id}
            initial={{ opacity: 0, y: 15 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.3, delay: index * 0.1 }}
            className="bg-[#0F0F0F] border border-white/10 p-6 shadow-2xl relative overflow-hidden rounded-none"
          >
            {/* Health Corner Indicator */}
            <div className="absolute top-4 right-4 flex items-center gap-2">
              <span className={`w-1.5 h-1.5 ${commitment.health === 'RED' ? 'hidden' : 'inline-block'} ${healthStyle.dot}`}></span>
              <span className={`text-[9px] font-mono tracking-widest ${healthStyle.text}`}>
                {commitment.health || 'PENDING'}
              </span>
            </div>

            {/* Commitment Header */}
            <div>
              <div className="flex items-center gap-2">
                <span className="text-[9px] font-mono font-bold tracking-[0.2em] uppercase text-white/40 bg-white/5 border border-white/10 px-2 py-0.5 rounded-none">
                  {commitment.commitment_type}
                </span>
                {commitment.health === 'RED' && (
                  <span className="text-[9px] font-mono font-bold text-black bg-[#FF4E00] px-2 py-0.5 rounded-none flex items-center gap-1 uppercase tracking-widest">
                    <Hourglass className="w-3 h-3 text-black shrink-0 animate-spin" />
                    VELOCITY_FALLOUT
                  </span>
                )}
              </div>
              <h3 className="font-display font-bold italic text-xl text-white mt-3">
                {commitment.title}
              </h3>
              <div className="flex items-center gap-3 mt-2 text-[10px] text-white/50 font-mono uppercase tracking-widest">
                <span className="flex items-center gap-1">
                  <Calendar className="w-3.5 h-3.5 text-white/30" />
                  Due: <strong className="text-white font-bold">{formatDeadline(commitment.deadline_date)}</strong>
                </span>
                <span className="text-white/20">|</span>
                <span>CONFIDENCE: {(commitment.confidence_score * 100).toFixed(0)}%</span>
              </div>
            </div>

            {/* Sub-actions checklist */}
            <div className="mt-6 border-t border-white/10 pt-5">
              <h4 className="text-[10px] uppercase tracking-[0.4em] text-white/40 mb-3">Project Execution Ledger</h4>
              <div className="space-y-3">
                {projectActions.map(action => (
                  <div
                    key={action.id}
                    className={`bg-black p-3 border transition-colors duration-200 rounded-none ${
                      action.status === 'COMPLETED' ? 'border-white/5 opacity-40' : 'border-white/10'
                    }`}
                  >
                    <div className="flex items-start gap-3">
                      <input
                        type="checkbox"
                        checked={action.status === 'COMPLETED'}
                        onChange={() => onToggleAction(action.id)}
                        className="mt-1 w-3.5 h-3.5 text-[#FF4E00] bg-black border-white/15 focus:ring-[#FF4E00] accent-[#FF4E00] cursor-pointer rounded-none"
                      />
                      <div className="flex-1 min-w-0">
                        <p className={`text-xs font-semibold text-white/80 ${action.status === 'COMPLETED' ? 'line-through text-white/30' : ''}`}>
                          {action.action_text}
                        </p>
                        <div className="flex items-center gap-2 mt-1.5 text-[9px] uppercase tracking-wider font-mono">
                          <span className="text-white/30">
                            Effort: {action.estimated_effort_hours} Hours
                          </span>
                          <span className="text-white/10">•</span>
                          <button
                            onClick={() => setExpandedActionId(expandedActionId === action.id ? null : action.id)}
                            className="text-[#FF4E00] hover:text-white font-mono font-bold flex items-center gap-0.5"
                          >
                            PRIORITY_INDEX: {action.priority_score.toFixed(1)}
                            {expandedActionId === action.id ? <ChevronUp className="w-3 h-3" /> : <ChevronDown className="w-3 h-3" />}
                          </button>
                        </div>
                      </div>
                    </div>

                    {/* Diagnostics card expansion */}
                    <AnimatePresence>
                      {expandedActionId === action.id && (
                        <motion.div
                          initial={{ height: 0, opacity: 0 }}
                          animate={{ height: 'auto', opacity: 1 }}
                          exit={{ height: 0, opacity: 0 }}
                          className="overflow-hidden mt-2.5 pt-2 border-t border-white/10 text-[10px] text-white/60 bg-[#121212] p-3 rounded-none font-sans"
                        >
                          <span className="font-mono uppercase tracking-widest text-[#FF4E00] font-bold block mb-1.5">Explainable Diagnostics:</span>
                          <ul className="list-disc pl-4 space-y-1 text-white/50">
                            {action.why_now_diagnostics?.map((diag, dIdx) => (
                              <li key={dIdx}>{diag}</li>
                            ))}
                          </ul>
                        </motion.div>
                      )}
                    </AnimatePresence>
                  </div>
                ))}
              </div>
            </div>

            {/* AI Recovery Plan Trigger & Plan View */}
            <div className="mt-6 border-t border-white/10 pt-5 flex flex-col gap-3">
              {commitment.health === 'RED' && !activePlan && (
                <div className="bg-[#FF4E00]/5 border border-[#FF4E00]/30 p-4 rounded-none text-xs flex gap-3">
                  <AlertOctagon className="w-5 h-5 text-[#FF4E00] shrink-0 mt-0.5" />
                  <div>
                    <h5 className="font-mono uppercase tracking-wider text-[#FF4E00] font-bold">Predictive Collapse Horizon</h5>
                    <p className="text-white/60 mt-1 leading-relaxed">
                      Core capacity modeling indicates critical project backlog. Initialize predictive neural recovery.
                    </p>
                  </div>
                </div>
              )}

              {/* Recovery Plan checklist */}
              {activePlan && (
                <div className="bg-[#121212] border border-white/10 p-4 rounded-none mt-2">
                  <div className="flex items-center gap-2 mb-3 border-b border-white/5 pb-2">
                    <Sparkles className="w-4 h-4 text-[#FF4E00]" />
                    <span className="text-[10px] uppercase tracking-[0.2em] font-bold text-white">RECOVERY_TRAJECTORY_PLAN</span>
                  </div>
                  <div className="space-y-2">
                    {activePlan.map((step: any, idx: number) => (
                      <div key={idx} className="flex items-center gap-3 bg-black p-2.5 border border-white/5 text-xs rounded-none font-mono uppercase tracking-wide">
                        <span className="font-bold text-[#FF4E00]">
                          {step.day}
                        </span>
                        <span className="flex-1 text-white/70 normal-case font-sans font-medium">{step.task}</span>
                        <span className="text-[9px] text-white/40 bg-white/5 px-2 py-0.5 border border-white/10">
                          {step.hours}H
                        </span>
                      </div>
                    ))}
                  </div>
                </div>
              )}

              <button
                disabled={loadingPlanId !== null}
                onClick={() => handleGeneratePlan(commitment.id)}
                className="w-full flex items-center justify-center gap-2 bg-[#FF4E00] hover:bg-white text-black hover:text-black text-[10px] uppercase tracking-[0.2em] font-black py-3 px-4 rounded-none transition cursor-pointer disabled:opacity-50 mt-1"
              >
                {loadingPlanId === commitment.id ? (
                  <>
                    <Loader2 className="w-4 h-4 animate-spin text-black" />
                    GENERATING_RECOVERY_TRAJECTORY...
                  </>
                ) : (
                  <>
                    <Sparkles className="w-4 h-4 text-black fill-black" />
                    SYNTHESIZE_RECOVERY_PLAN
                  </>
                )}
              </button>
            </div>
          </motion.div>
        );
      })}
    </div>
  );
}
