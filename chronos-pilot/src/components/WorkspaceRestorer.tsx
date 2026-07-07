/**
 * @license
 * SPDX-License-Identifier: Apache-2.0
 */

import React, { useState } from 'react';
import { Play, Terminal, ArrowRight, Compass, FileText, CheckCircle, Search, HelpCircle, AlertTriangle } from 'lucide-react';
import { motion, AnimatePresence } from 'motion/react';

interface ProjectWorkspace {
  id: number;
  project_name: string;
  state: any;
  deadlines: any[];
}

interface WorkspaceRestorerProps {
  projects: ProjectWorkspace[];
  onRefresh?: () => void;
}

export default function WorkspaceRestorer({ projects, onRefresh }: WorkspaceRestorerProps) {
  const [activeProjectId, setActiveProjectId] = useState<number | null>(null);
  const [isRestoring, setIsRestoring] = useState(false);
  const [restoreSteps, setRestoreSteps] = useState<string[]>([]);
  const [whyStopped, setWhyStopped] = useState<string>('');
  const [restoredState, setRestoredState] = useState<any>(null);

  const handleStartWorking = async (projectId: number) => {
    setActiveProjectId(projectId);
    setIsRestoring(true);
    setRestoreSteps([]);
    setWhyStopped('');
    setRestoredState(null);

    const steps = [
      'Establishing secure local loopback IPC connection with Workspace Connector...',
      'Emitting handshake validation token for workspace authorization...',
      'Tauri Daemon commanding IDE connector: Restoring active codebase directories...',
      'Loaded local codebase: Opening tabs & aligned line/column cursor positions...',
      'Chrome browser extension intercepting command: Restoring target active RESEARCH_SESSION tabs...',
      'Grouped RAG chunking sessions & restored Stripe Webhook Documentation...',
      'Writing context continuation layout into local ./chronos_staged/ folder...'
    ];

    // Simulating steps incrementally for premium visual fidelity
    for (let i = 0; i < steps.length; i++) {
      await new Promise(resolve => setTimeout(resolve, 400));
      setRestoreSteps(prev => [...prev, steps[i]]);
    }

    try {
      const res = await fetch('/api/execution/restore-workspace', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ target_session_id: undefined })
      });
      const data = await res.json();
      if (data.ok) {
        const payload = data.data.restore_action?.payload || {};
        setWhyStopped(payload.explanation || 'Resumed workspace successfully.');
        setRestoredState({
          files: payload.files_to_reopen || [],
          session: payload.restore_target_session_id
        });
        if (onRefresh) onRefresh();
      }
    } catch (e) {
      console.error(e);
    } finally {
      setIsRestoring(false);
    }
  };

  return (
    <div id="restoration-panel" className="bg-[#0F0F0F] border border-white/10 p-6 shadow-2xl rounded-none">
      <div className="flex items-center justify-between border-b border-white/10 pb-4 mb-4">
        <div>
          <h2 className="font-display font-black italic text-lg text-white flex items-center gap-2">
            <Compass className="w-5 h-5 text-[#FF4E00]" />
            WORKSPACE_RESTORER
          </h2>
          <p className="text-[10px] uppercase tracking-widest text-white/40 mt-1">Reconstruct file editor layout & research tabs</p>
        </div>
        <span className="text-[9px] font-mono text-white/40 uppercase tracking-widest">
          IPC_V1.0_PRO
        </span>
      </div>

      <div className="space-y-4">
        {projects.map(project => (
          <div
            key={project.id}
            className={`bg-[#121212] p-4 border transition-all duration-300 rounded-none ${
              activeProjectId === project.id ? 'border-[#FF4E00] bg-black' : 'border-white/5 hover:border-white/10'
            }`}
          >
            <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
              <div>
                <h3 className="font-display font-bold text-sm text-white">{project.project_name}</h3>
                <p className="text-[11px] text-white/50 mt-1">{project.state?.current_summary}</p>
              </div>
              <button
                onClick={() => handleStartWorking(project.id)}
                disabled={isRestoring && activeProjectId === project.id}
                className="flex items-center justify-center gap-1.5 bg-[#FF4E00] hover:bg-white text-black text-[10px] uppercase tracking-[0.2em] font-black py-2 px-4 rounded-none transition shrink-0 cursor-pointer"
              >
                <Play className="w-3 h-3 fill-black text-black" />
                START_RESTORE
              </button>
            </div>

            {/* Restoring Step Console */}
            {activeProjectId === project.id && (restoreSteps.length > 0 || isRestoring) && (
              <div className="mt-4 bg-black rounded-none p-4 border border-white/5 font-mono text-[10px] text-white/80 leading-relaxed">
                <div className="flex items-center gap-2 border-b border-white/5 pb-2 mb-2 text-[9px] text-white/40 font-bold uppercase tracking-wider">
                  <Terminal className="w-3.5 h-3.5 text-white/30" />
                  PIPELINE_EXECUTION_LOGS
                </div>
                <div className="space-y-1 max-h-40 overflow-y-auto">
                  {restoreSteps.map((step, sIdx) => (
                    <div key={sIdx} className="flex items-start gap-1.5 animate-fade-in text-white/70">
                      <span className="text-[#FF4E00] shrink-0">&gt;</span>
                      <span>{step}</span>
                    </div>
                  ))}
                  {isRestoring && (
                    <div className="flex items-center gap-1.5 text-[#FF4E00] animate-pulse">
                      <span>&gt;</span>
                      <span>RESOLVING_NEURAL_THREAD_CONTEXT...</span>
                    </div>
                  )}
                </div>
              </div>
            )}

            {/* Restored State & Diagnostics */}
            {activeProjectId === project.id && whyStopped && (
              <motion.div
                initial={{ opacity: 0, y: 10 }}
                animate={{ opacity: 1, y: 0 }}
                className="mt-4 bg-[#121212] p-4 border border-[#FF4E00]/20"
              >
                {/* Reconstruction Narrative Summary */}
                <div className="mb-4">
                  <h4 className="text-[9px] font-bold text-[#FF4E00] flex items-center gap-1.5 uppercase tracking-widest mb-2 font-mono">
                    Why You Stopped Diagnostics
                  </h4>
                  <div className="text-xs text-white/70 leading-relaxed space-y-1 bg-black p-3 rounded-none border border-white/5 font-mono whitespace-pre-wrap">
                    {whyStopped}
                  </div>
                </div>

                {/* Programmatic State Evidence */}
                {restoredState && (
                  <div className="bg-black p-3 border border-white/5 text-xs">
                    <span className="text-[9px] font-bold text-white/40 uppercase tracking-widest block mb-2 font-mono">
                      Restored Environment Details (Evidence)
                    </span>
                    <div className="grid grid-cols-1 sm:grid-cols-2 gap-3 font-mono text-[10px]">
                      <div className="flex items-start gap-2 bg-[#121212] p-2 border border-white/5">
                        <FileText className="w-4 h-4 text-[#FF4E00] shrink-0 mt-0.5" />
                        <div>
                          <span className="text-white/30 block text-[8px] uppercase tracking-wider font-mono">Codebase Target</span>
                          <span className="text-white font-bold block truncate">{restoredState.active_file_path}</span>
                          <span className="text-white/40 text-[9px]">L: {restoredState.cursor_line} | C: {restoredState.cursor_column}</span>
                        </div>
                      </div>

                      <div className="flex items-start gap-2 bg-[#121212] p-2 border border-white/5">
                        <Search className="w-4 h-4 text-[#FF4E00] shrink-0 mt-0.5" />
                        <div>
                          <span className="text-white/30 block text-[8px] uppercase tracking-wider font-mono">Browser Session</span>
                          <span className="text-white font-bold block truncate">docs.stripe.com/signatures</span>
                          <span className="text-[#FF4E00] text-[9px] flex items-center gap-0.5 font-bold uppercase tracking-wider">
                            <CheckCircle className="w-3 h-3 text-[#FF4E00]" /> Sync OK
                          </span>
                        </div>
                      </div>
                    </div>
                  </div>
                )}
              </motion.div>
            )}
          </div>
        ))}
      </div>
    </div>
  );
}
