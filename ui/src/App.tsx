/**
 * @license
 * SPDX-License-Identifier: Apache-2.0
 */

import React, { useState, useEffect } from 'react';
import { API_BASE } from './config';
import { 
  Compass, 
  Sparkles, 
  Terminal, 
  Shield, 
  RefreshCw, 
  AlertTriangle, 
  Clock, 
  Layers, 
  GitCommit, 
  PlayCircle, 
  LayoutList, 
  Cpu, 
  ArrowRight, 
  CheckCircle2, 
  Eye, 
  EyeOff,
  Flame, 
  Activity, 
  BookOpen, 
  Binary,
  LayoutGrid,
  Sliders,
  Play,
  CheckCircle,
  CheckSquare,
  HelpCircle,
  ArrowUpRight,
  TrendingUp,
  Search,
  FileText,
  Sun,
  Moon
} from 'lucide-react';
import { motion, AnimatePresence } from 'motion/react';
import InteractiveSandbox from './components/InteractiveSandbox';
import RiskForecaster from './components/RiskForecaster';
import CommitmentList from './components/CommitmentList';
import WorkspaceRestorer from './components/WorkspaceRestorer';
import ARCPanel from './components/ARCPanel';
import DatabaseViewer from './components/DatabaseViewer';
import FlightRecorderPanel from './components/FlightRecorderPanel';
import { Commitment, ProjectAction, TelemetryLog } from './types';

// redefined ViewMode based on the three fundamental modes of Chronos
type ViewMode = 'invisible' | 'passive' | 'intervention';

export default function App() {
  const [systemTime, setSystemTime] = useState<string>('2026-06-23T10:15:00-07:00');
  const [hasApiKey, setHasApiKey] = useState(false);
  const [commitments, setCommitments] = useState<Commitment[]>([]);
  const [actions, setActions] = useState<ProjectAction[]>([]);
  const [projects, setProjects] = useState<any[]>([]);
  const [refreshTrigger, setRefreshTrigger] = useState(0);
  const [activeMode, setActiveMode] = useState<ViewMode>('passive');
  const [isLightMode, setIsLightMode] = useState(false);
  const [cpuUsage, setCpuUsage] = useState<number>(0);
  const [ramUsage, setRamUsage] = useState<number>(0);
  const [batteryPercentage, setBatteryPercentage] = useState<number | null>(null);
  const [gatekeeperPaused, setGatekeeperPaused] = useState<boolean>(false);

  useEffect(() => {
    if (isLightMode) {
      document.documentElement.classList.add('light-mode');
    } else {
      document.documentElement.classList.remove('light-mode');
    }
  }, [isLightMode]);
  const [narrative, setNarrative] = useState<string>('No diagnostic available.');

  // Sliders for Mode 3: Intervention Simulator
  const [daysUntouched, setDaysUntouched] = useState<number>(11);
  const [deadlineHours, setDeadlineHours] = useState<number>(48);
  const [commitmentDrift, setCommitmentDrift] = useState<number>(5);
  const [focusIntensity, setFocusIntensity] = useState<number>(8);

  // Mode 1: Simulated Flight Recorder Logs
  const [flightLogs, setFlightLogs] = useState<TelemetryLog[]>([]);

  const fetchData = async () => {
    try {
      // 1. Fetch system status
      const statusRes = await fetch(`${API_BASE}/api/system-status`);
      const statusData = await statusRes.json();
      setSystemTime(statusData.systemTime);
      setHasApiKey(statusData.hasApiKey);
      setNarrative(statusData.narrative || 'No diagnostic available.');
      setCpuUsage(statusData.cpuUsage || 0);
      setRamUsage(statusData.ramUsage || 0);
      setBatteryPercentage(statusData.batteryPercentage ?? null);
      setGatekeeperPaused(statusData.gatekeeperPaused || false);

      // 2. Fetch commitments
      const commitmentsRes = await fetch(`${API_BASE}/api/commitments`);
      const commitmentsData = await commitmentsRes.json();
      setCommitments(commitmentsData);

      // 3. Fetch actions
      const actionsRes = await fetch(`${API_BASE}/api/actions`);
      const actionsData = await actionsRes.json();
      setActions(actionsData);

      // 4. Fetch projects
      const projectsRes = await fetch(`${API_BASE}/api/projects`);
      const projectsData = await projectsRes.json();
      setProjects(projectsData);
    } catch (e) {
      console.error('Error fetching Chronos Pilot data:', e);
    }
  };

  const fetchTelemetryLogs = async () => {
    try {
      const res = await fetch(`${API_BASE}/api/telemetry-logs`);
      const logs = await res.json();
      setFlightLogs(prev => {
        if (!prev) return logs;
        return JSON.stringify(prev) === JSON.stringify(logs) ? prev : logs;
      });
    } catch (e) {
      console.error('Error fetching telemetry logs:', e);
    }
  };

  useEffect(() => {
    fetchTelemetryLogs();
    const interval = setInterval(fetchTelemetryLogs, 2000);
    return () => clearInterval(interval);
  }, []);

  useEffect(() => {
    fetchData();
  }, [refreshTrigger]);

  const handleRefresh = () => {
    setRefreshTrigger(prev => prev + 1);
    fetchTelemetryLogs();
  };

  const handleTimeTravel = async (hours: number) => {
    try {
      const res = await fetch(`${API_BASE}/api/time-travel`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ hours })
      });
      const data = await res.json();
      if (data.success) {
        handleRefresh();
      }
    } catch (e) {
      console.error(e);
    }
  };

  const handleToggleAction = async (actionId: number) => {
    try {
      const res = await fetch(`${API_BASE}/api/actions/toggle`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ action_id: actionId })
      });
      const data = await res.json();
      if (data.success) {
        handleRefresh();
      }
    } catch (e) {
      console.error(e);
    }
  };

  // Math variables for the Decision Engine Simulator
  const silenceCost = Number((daysUntouched * 4.5 + commitmentDrift * 8.0 + (120 - deadlineHours) * 0.6).toFixed(1));
  const interruptionCost = Number((focusIntensity * 2.5 + 12).toFixed(1));
  const shouldIntervene = silenceCost > interruptionCost;

  const anyRedCommitment = commitments.some(c => c.health === 'RED');

  return (
    <div className="min-h-screen bg-[#070707] text-[#F4F4F1] pb-16 select-none antialiased font-sans">
      
      {/* Dynamic header / warning banner */}
      {anyRedCommitment && (
        <div className="bg-[#FF4E00] text-black px-4 py-2 text-center text-xs flex items-center justify-center gap-2 font-mono uppercase tracking-[0.2em] font-black animate-pulse">
          <AlertTriangle className="w-4 h-4 text-black shrink-0" />
          <span>[SYSTEM CRITICAL] CHRONOS FORECAST ALERT: COMMITMENTS ENTERED VELOCITY COLLAPSE MARGIN</span>
        </div>
      )}

      {/* Primary Dashboard Navigation Banner */}
      <header className="h-24 border-b border-white/10 bg-[#0F0F0F] flex items-center justify-between px-6 md:px-12 sticky top-0 z-40">
        <div className="flex items-center gap-4">
          <div className="border border-[#FF4E00] p-1.5 bg-[#FF4E00]/10 flex items-center justify-center">
            <Compass className="w-5 h-5 text-[#FF4E00] animate-spin" style={{ animationDuration: '25s' }} />
          </div>
          <div>
            <div className="flex items-center gap-2">
              <h1 className="font-display font-black italic text-xl tracking-tighter text-white">
                CHRONOS_PILOT
              </h1>
              <span className="text-[8px] font-mono font-bold border border-[#FF4E00]/40 text-[#FF4E00] px-1.5 py-0.5 rounded tracking-[0.2em]">
                ACTIVE_SYS
              </span>
            </div>
            <p className="text-[10px] uppercase tracking-widest text-white/40 mt-0.5 font-mono">Personal Execution Operating System</p>
          </div>
        </div>

        {/* Global HUD Status details */}
        <div className="flex items-center gap-6 text-[10px] uppercase tracking-widest font-mono">
          <div className="hidden lg:flex items-center gap-2 text-white/50">
            <span className="w-1.5 h-1.5 bg-[#FF4E00]"></span>
            <span>Local Sovereign Loopback</span>
          </div>
          
          <div className="hidden md:flex items-center gap-2 text-white/50">
            <Clock className="w-3.5 h-3.5 text-white/40" />
            <span>TIME: {systemTime.replace('T', ' ').substring(0, 19)}</span>
          </div>

          <div className="hidden xl:flex items-center gap-4 border-l border-white/10 pl-4">
            <div className="flex items-center gap-1.5 text-white/50">
              <Cpu className="w-3.5 h-3.5 text-cyan-400" />
              <span>CPU: {cpuUsage.toFixed(0)}%</span>
            </div>

            <div className="flex items-center gap-1.5 text-white/50">
              <span className="text-purple-400 font-bold">RAM:</span>
              <span>{ramUsage.toFixed(0)}%</span>
            </div>

            {batteryPercentage !== null && (
              <div className="flex items-center gap-1.5 text-white/50">
                <span className={`w-1.5 h-1.5 ${batteryPercentage < 20 ? 'bg-red-500 animate-pulse' : 'bg-emerald-500'}`}></span>
                <span>BAT: {batteryPercentage}%</span>
              </div>
            )}

            <div className="flex items-center gap-2">
              <span className={`w-1.5 h-1.5 rounded-full ${gatekeeperPaused ? 'bg-red-500 animate-pulse' : 'bg-emerald-500 shadow-[0_0_6px_rgba(16,185,129,0.5)]'}`}></span>
              <span className={gatekeeperPaused ? 'text-red-400 font-bold text-[9px]' : 'text-emerald-400 text-[9px]'}>
                {gatekeeperPaused ? 'THROTTLED' : 'GATEKEEPER'}
              </span>
            </div>
          </div>

          <button
            onClick={() => setIsLightMode(!isLightMode)}
            className="border border-white/10 hover:border-white/30 bg-white/5 text-white p-2 transition cursor-pointer"
            title="Toggle Light/Dark Mode"
          >
            {isLightMode ? <Moon className="w-3.5 h-3.5" /> : <Sun className="w-3.5 h-3.5" />}
          </button>

          <button
            onClick={handleRefresh}
            className="border border-white/10 hover:border-white/30 bg-white/5 text-white p-2 transition cursor-pointer"
            title="Refresh System"
          >
            <RefreshCw className="w-3.5 h-3.5" />
          </button>
        </div>
      </header>

      {/* Main Container */}
      <main className="max-w-7xl mx-auto px-6 md:px-12 mt-8">
        
        {/* CHRONOS OPERATIONAL MODE SELECTOR */}
        <section className="mb-8 bg-black border border-white/10 p-5 shadow-xl">
          <div className="flex flex-col sm:flex-row sm:items-center justify-between border-b border-white/5 pb-3 mb-4 gap-2">
            <div>
              <span className="text-[9px] uppercase tracking-[0.3em] text-[#FF4E00] font-mono font-bold">Chronos Mode Configuration</span>
              <h3 className="text-sm font-bold text-white uppercase font-mono tracking-wider mt-0.5">Select Active View Pipeline</h3>
            </div>
            <span className="text-[9px] font-mono text-white/30 uppercase tracking-widest">
              Zero-Maintenance Principle Enabled
            </span>
          </div>

          <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
            
            {/* Mode 1: Invisible */}
            <button
              onClick={() => setActiveMode('invisible')}
              className={`text-left p-4 border transition-all cursor-pointer flex flex-col justify-between relative overflow-hidden h-28 ${
                activeMode === 'invisible' 
                  ? 'border-[#FF4E00] bg-[#FF4E00]/5' 
                  : 'border-white/5 bg-[#0F0F0F]/80 hover:border-white/20 hover:bg-white/5'
              }`}
            >
              <div className="flex items-center justify-between w-full">
                <span className="text-[10px] font-mono text-[#FF4E00] font-bold">MODE 01 (95%)</span>
                <EyeOff className={`w-4 h-4 ${activeMode === 'invisible' ? 'text-[#FF4E00]' : 'text-white/40'}`} />
              </div>
              <div className="mt-2">
                <span className="text-xs font-bold font-mono text-white block tracking-wider">
                  INVISIBLE FLIGHT RECORDER
                </span>
                <span className="text-[9px] text-white/40 block mt-1 leading-snug font-mono">
                  Sits quietly, logging documents, files, and browser check-ins with zero interruptions.
                </span>
              </div>
              {activeMode === 'invisible' && (
                <div className="absolute right-0 bottom-0 w-2 h-2 bg-[#FF4E00]" />
              )}
            </button>

            {/* Mode 2: Passive */}
            <button
              onClick={() => setActiveMode('passive')}
              className={`text-left p-4 border transition-all cursor-pointer flex flex-col justify-between relative overflow-hidden h-28 ${
                activeMode === 'passive' 
                  ? 'border-[#FF4E00] bg-[#FF4E00]/5' 
                  : 'border-white/5 bg-[#0F0F0F]/80 hover:border-white/20 hover:bg-white/5'
              }`}
            >
              <div className="flex items-center justify-between w-full">
                <span className="text-[10px] font-mono text-[#FF4E00] font-bold">MODE 02 (4%)</span>
                <LayoutGrid className={`w-4 h-4 ${activeMode === 'passive' ? 'text-[#FF4E00]' : 'text-white/40'}`} />
              </div>
              <div className="mt-2">
                <span className="text-xs font-bold font-mono text-white block tracking-wider">
                  PASSIVE MISSION CONTROL
                </span>
                <span className="text-[9px] text-white/40 block mt-1 leading-snug font-mono">
                  The voluntary single-screen cockpit answering what is at risk and how to restore workspace.
                </span>
              </div>
              {activeMode === 'passive' && (
                <div className="absolute right-0 bottom-0 w-2 h-2 bg-[#FF4E00]" />
              )}
            </button>

            {/* Mode 3: Intervention */}
            <button
              onClick={() => setActiveMode('intervention')}
              className={`text-left p-4 border transition-all cursor-pointer flex flex-col justify-between relative overflow-hidden h-28 ${
                activeMode === 'intervention' 
                  ? 'border-[#FF4E00] bg-[#FF4E00]/5' 
                  : 'border-white/5 bg-[#0F0F0F]/80 hover:border-white/20 hover:bg-white/5'
              }`}
            >
              <div className="flex items-center justify-between w-full">
                <span className="text-[10px] font-mono text-[#FF4E00] font-bold">MODE 03 (1%)</span>
                <Sliders className={`w-4 h-4 ${activeMode === 'intervention' ? 'text-[#FF4E00]' : 'text-white/40'}`} />
              </div>
              <div className="mt-2">
                <span className="text-xs font-bold font-mono text-white block tracking-wider">
                  INTERVENTION DECISION ENGINE
                </span>
                <span className="text-[9px] text-white/40 block mt-1 leading-snug font-mono">
                  Triggers rare critical alerts only when context collapse cost exceeds distraction cost.
                </span>
              </div>
              {activeMode === 'intervention' && (
                <div className="absolute right-0 bottom-0 w-2 h-2 bg-[#FF4E00]" />
              )}
            </button>
          </div>
          
          <div className="mt-3 bg-[#111] px-4 py-2 border border-white/5 text-[10px] font-mono text-white/60 uppercase tracking-wider flex items-center justify-between">
            <div className="flex items-center gap-2">
              <span className="text-[#FF4E00] font-bold">&gt;&gt; LAW OF RECOVERY:</span>
              <span>"Chronos must never require maintenance. Every interaction must reduce administrative context debt more than it creates."</span>
            </div>
            <span className="text-white/30 hidden md:inline">SYSTEM STATE: ACTIVE</span>
          </div>
        </section>

        {/* API Key Instructions */}
        {!hasApiKey && (
          <div className="border border-[#FF4E00]/30 bg-[#FF4E00]/5 p-4 mb-8 text-xs flex gap-3 text-slate-300">
            <Sparkles className="w-5 h-5 text-[#FF4E00] shrink-0 mt-0.5" />
            <div>
              <span className="font-mono uppercase text-[#FF4E00] font-bold block mb-1">PROACTIVE AI INFERENCE OFFLINE</span>
              To unlock premium AI-generated catch-up schedules, checkpoint diagnostics, and cognitive research summaries, configure your Gemini API Key inside the <strong className="text-white">Settings &gt; Secrets</strong> panel in the AI Studio sidebar.
            </div>
          </div>
        )}

        {/* RENDERING DYNAMIC MODES */}
        <AnimatePresence mode="wait">
          <motion.div
            key={activeMode}
            initial={{ opacity: 0, y: 10 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: -10 }}
            transition={{ duration: 0.25 }}
          >
            
            {/* ================= MODE 1: INVISIBLE DAEMON (95%) ================= */}
            {activeMode === 'invisible' && (
              <FlightRecorderPanel
                systemTime={systemTime}
                hasApiKey={hasApiKey}
                flightLogs={flightLogs}
                simulatingLog={false}
                onRefresh={handleRefresh}
                onTimeTravel={handleTimeTravel}
              />
            )}

            {/* ================= MODE 2: PASSIVE MISSION CONTROL (4%) ================= */}
            {activeMode === 'passive' && (
              <div className="space-y-8">
                
                {/* Intro status line */}
                <div className="flex flex-col sm:flex-row items-start sm:items-center justify-between bg-black border border-white/5 px-5 py-3 gap-2 font-mono text-[10px]">
                  <div className="flex items-center gap-2">
                    <span className="text-[#FF4E00] font-black">● PASSIVE STATE</span>
                    <span className="text-white/40">VOLUNTARY INSPECTION OPEN</span>
                  </div>
                  <span className="text-white/30 uppercase tracking-widest">NO CHATTER. JUST RAW CONTEXT EVIDENCE.</span>
                </div>

                {/* 4 QUADRANTS GRID */}
                <div className="grid grid-cols-1 lg:grid-cols-2 gap-8">
                  
                  {/* QUADRANT A: What is at risk? */}
                  <div className="bg-[#0F0F0F] border border-white/10 p-6 flex flex-col justify-between">
                    <div className="border-b border-white/10 pb-3 mb-4">
                      <span className="text-[9px] font-mono uppercase tracking-[0.3em] text-[#FF4E00] block">QUADRANT_01 / RISK ANALYSIS</span>
                      <h3 className="text-base font-bold text-white uppercase mt-1 font-mono tracking-wider flex items-center gap-2">
                        <TrendingUp className="w-4 h-4 text-[#FF4E00]" />
                        What is at risk?
                      </h3>
                    </div>
                    <div className="flex-1">
                      <RiskForecaster commitments={commitments} />
                    </div>
                  </div>

                  {/* QUADRANT B: What should I do next? */}
                  <div className="bg-[#0F0F0F] border border-white/10 p-6 flex flex-col justify-between">
                    <div className="border-b border-white/10 pb-3 mb-4">
                      <span className="text-[9px] font-mono uppercase tracking-[0.3em] text-[#FF4E00] block">QUADRANT_02 / STRATEGY LEDGER</span>
                      <h3 className="text-base font-bold text-white uppercase mt-1 font-mono tracking-wider flex items-center gap-2">
                        <CheckSquare className="w-4 h-4 text-[#FF4E00]" />
                        What should I do next?
                      </h3>
                    </div>
                    <div className="flex-1 bg-black p-4 border border-white/5">
                      <div className="mb-4 text-xs font-mono text-white/50 leading-relaxed uppercase border-b border-white/5 pb-2">
                        &gt; Active context continuation plans from files:
                      </div>
                      <CommitmentList
                        commitments={commitments}
                        actions={actions}
                        onToggleAction={handleToggleAction}
                        onPlanGenerated={handleRefresh}
                      />
                    </div>
                  </div>

                  {/* QUADRANT C: Why did I stop? */}
                  <div className="bg-[#0F0F0F] border border-white/10 p-6 flex flex-col justify-between">
                    <div className="border-b border-white/10 pb-3 mb-4">
                      <span className="text-[9px] font-mono uppercase tracking-[0.3em] text-[#FF4E00] block">QUADRANT_03 / COGNITIVE FORENSICS</span>
                      <h3 className="text-base font-bold text-white uppercase mt-1 font-mono tracking-wider flex items-center gap-2">
                        <FileText className="w-4 h-4 text-[#FF4E00]" />
                        Why did I stop?
                      </h3>
                    </div>
                    <div className="flex-1 space-y-4">
                      <div className="bg-black border border-white/5 p-4 rounded-none">
                        <div className="flex items-center gap-2 text-[#FF4E00] text-[10px] font-bold font-mono uppercase tracking-wider mb-2">
                          <AlertTriangle className="w-4 h-4" />
                          Context Blockage Diagnostic
                        </div>
                        <p className="text-xs text-white/80 font-mono leading-relaxed bg-[#111] p-3 border border-white/5 whitespace-pre-wrap">
                          {narrative}
                        </p>
                      </div>
                    </div>
                  </div>

                  {/* QUADRANT D: Restore workspace */}
                  <div className="bg-[#0F0F0F] border border-white/10 p-6 flex flex-col justify-between">
                    <div className="border-b border-white/10 pb-3 mb-4">
                      <span className="text-[9px] font-mono uppercase tracking-[0.3em] text-[#FF4E00] block">QUADRANT_04 / STATE RESUMPTION</span>
                      <h3 className="text-base font-bold text-white uppercase mt-1 font-mono tracking-wider flex items-center gap-2">
                        <Layers className="w-4 h-4 text-[#FF4E00]" />
                        Restore workspace
                      </h3>
                    </div>
                    <div className="flex-1">
                      <WorkspaceRestorer projects={projects} />
                    </div>
                  </div>

                  {/* QUADRANT E: Autonomous Research Crawler (ARC) */}
                  <div className="bg-[#0F0F0F] border border-white/10 p-6 flex flex-col justify-between col-span-1 lg:col-span-2">
                    <div className="border-b border-white/10 pb-3 mb-4">
                      <span className="text-[9px] font-mono uppercase tracking-[0.3em] text-[#FF4E00] block">QUADRANT_05 / AUTONOMOUS RESEARCH CRAWLER (ARC)</span>
                      <h3 className="text-base font-bold text-white uppercase mt-1 font-mono tracking-wider flex items-center gap-2">
                        <Search className="w-4 h-4 text-[#FF4E00]" />
                        Active Background Intelligence
                      </h3>
                    </div>
                    <div className="flex-1">
                      <ARCPanel />
                    </div>
                  </div>

                </div>

                {/* Sovereignty verification database audit tool */}
                <div className="pt-6 border-t border-white/10">
                  <DatabaseViewer
                    systemTime={systemTime}
                    refreshTrigger={refreshTrigger}
                    onRefresh={handleRefresh}
                  />
                </div>
              </div>
            )}

            {/* ================= MODE 3: INTERVENTION ENGINE (1%) ================= */}
            {activeMode === 'intervention' && (
              <div className="space-y-8">
                
                {/* Mathematical Engine Simulator Console */}
                <div className="bg-black border border-white/10 p-6 shadow-2xl">
                  <span className="text-[10px] font-mono uppercase tracking-[0.3em] text-[#FF4E00] block font-bold">CHRONOS DECISION THEORY SIMULATOR</span>
                  <h2 className="text-lg font-bold font-mono text-white mt-1 uppercase border-b border-white/5 pb-2">
                    Cost of Silence vs Cost of Interruption
                  </h2>
                  <p className="text-xs text-white/50 mt-2 leading-relaxed">
                    Chronos evaluates active workflow metrics continually. It intervenes only when <strong className="text-[#FF4E00]">Silence Cost &gt; Interruption Cost</strong>. Adjust the variables below to test and simulate exactly when Chronos decides to trigger an intervention alert.
                  </p>

                  <div className="grid grid-cols-1 md:grid-cols-4 gap-6 mt-6 bg-[#0F0F0F] p-5 border border-white/5">
                    
                    {/* Days untouched */}
                    <div className="space-y-2">
                      <div className="flex justify-between font-mono text-[10px]">
                        <span className="text-white/50">DAYS UNTOUCHED:</span>
                        <span className="text-[#FF4E00] font-bold">{daysUntouched} DAYS</span>
                      </div>
                      <input 
                        type="range" min="0" max="30" value={daysUntouched}
                        onChange={(e) => setDaysUntouched(Number(e.target.value))}
                        className="w-full accent-[#FF4E00] cursor-pointer"
                      />
                      <span className="text-[8px] text-white/30 block font-mono">Simulates context decay duration</span>
                    </div>

                    {/* Deadline Hours */}
                    <div className="space-y-2">
                      <div className="flex justify-between font-mono text-[10px]">
                        <span className="text-white/50">DEADLINE IN:</span>
                        <span className="text-[#FF4E00] font-bold">{deadlineHours} HOURS</span>
                      </div>
                      <input 
                        type="range" min="2" max="120" value={deadlineHours}
                        onChange={(e) => setDeadlineHours(Number(e.target.value))}
                        className="w-full accent-[#FF4E00] cursor-pointer"
                      />
                      <span className="text-[8px] text-white/30 block font-mono">Simulates proximity to due date</span>
                    </div>

                    {/* Commitment Drift */}
                    <div className="space-y-2">
                      <div className="flex justify-between font-mono text-[10px]">
                        <span className="text-white/50">COMMITMENT DRIFT:</span>
                        <span className="text-[#FF4E00] font-bold">{commitmentDrift} DAYS</span>
                      </div>
                      <input 
                        type="range" min="0" max="15" value={commitmentDrift}
                        onChange={(e) => setCommitmentDrift(Number(e.target.value))}
                        className="w-full accent-[#FF4E00] cursor-pointer"
                      />
                      <span className="text-[8px] text-white/30 block font-mono">Days since committing but no activity</span>
                    </div>

                    {/* Focus level */}
                    <div className="space-y-2">
                      <div className="flex justify-between font-mono text-[10px]">
                        <span className="text-white/50">USER WORK FOCUS:</span>
                        <span className="text-[#FF4E00] font-bold">LEVEL {focusIntensity}/10</span>
                      </div>
                      <input 
                        type="range" min="1" max="10" value={focusIntensity}
                        onChange={(e) => setFocusIntensity(Number(e.target.value))}
                        className="w-full accent-[#FF4E00] cursor-pointer"
                      />
                      <span className="text-[8px] text-white/30 block font-mono">User activity level (higher = harder to interrupt)</span>
                    </div>
                  </div>

                  {/* Decision engine result display */}
                  <div className="mt-6 grid grid-cols-1 md:grid-cols-3 gap-4 border-t border-white/5 pt-6">
                    <div className="bg-black p-4 border border-white/5 flex flex-col justify-between">
                      <span className="text-[9px] font-mono text-white/40 uppercase block">SILENCE_COST (Accumulating context debt)</span>
                      <span className="text-3xl font-display font-black italic text-[#FF4E00] mt-1">{silenceCost}</span>
                      <span className="text-[8px] font-mono text-white/20 mt-1">Days*{daysUntouched ? '4.5' : '0'} + Drift*{commitmentDrift ? '8.0' : '0'} + Hours*{deadlineHours ? '0.6' : '0'}</span>
                    </div>

                    <div className="bg-black p-4 border border-white/5 flex flex-col justify-between">
                      <span className="text-[9px] font-mono text-white/40 uppercase block">INTERRUPTION_COST (Price of distraction)</span>
                      <span className="text-3xl font-display font-black italic text-white mt-1">{interruptionCost}</span>
                      <span className="text-[8px] font-mono text-white/20 mt-1">Focus*{focusIntensity ? '2.5' : '0'} + Base(12)</span>
                    </div>

                    <div className={`p-4 border flex flex-col justify-between uppercase font-mono ${
                      shouldIntervene 
                        ? 'bg-red-950/20 border-red-500/50 text-red-500' 
                        : 'bg-green-950/10 border-green-500/30 text-green-400'
                    }`}>
                      <span className="text-[9px] block">ENGINE OUTCOME STATUS</span>
                      <div className="flex items-center gap-1.5 mt-2 font-black italic text-base">
                        {shouldIntervene ? (
                          <>
                            <AlertTriangle className="w-5 h-5 animate-pulse" />
                            <span>BREACHED: INTERVENE!</span>
                          </>
                        ) : (
                          <>
                            <EyeOff className="w-5 h-5" />
                            <span>STAY INVISIBLE (SILENT)</span>
                          </>
                        )}
                      </div>
                      <span className="text-[8px] block mt-1">
                        {shouldIntervene ? 'Silence Cost exceeds Interruption Cost.' : 'Protected user flow state.'}
                      </span>
                    </div>
                  </div>
                </div>

                {/* THE INTERVENE ALERTS BLOCK */}
                <div className="space-y-6">
                  <div className="flex items-center gap-2 border-b border-white/10 pb-3">
                    <AlertTriangle className="text-[#FF4E00] w-5 h-5 shrink-0" />
                    <h3 className="text-base font-bold text-white font-mono uppercase tracking-widest">
                      Rare Intervention Deck (Simulated Outcome)
                    </h3>
                  </div>

                  {shouldIntervene ? (
                    <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
                      
                      {/* Alert Card 1: Deadline Risk */}
                      <motion.div
                        initial={{ opacity: 0, scale: 0.98 }}
                        animate={{ opacity: 1, scale: 1 }}
                        className="bg-black border-2 border-red-500 p-6 flex flex-col justify-between h-80 relative"
                      >
                        <div className="absolute top-4 right-4 bg-red-500 text-black font-mono font-black text-[9px] px-2 py-0.5 uppercase tracking-widest">
                          RISK LEVEL 01
                        </div>
                        <div>
                          <span className="text-[9px] font-mono text-red-500 uppercase tracking-widest block font-bold">CHRONOS INTERVENE ALERT</span>
                          <h4 className="text-lg font-bold font-mono text-white uppercase mt-2">DEADLINE COHERENCE BREACH</h4>
                          <p className="text-xs text-white/70 mt-3 leading-relaxed font-mono bg-red-500/5 p-3 border border-red-500/20">
                            Assignment due in <strong className="text-white">{deadlineHours} hours</strong>. 
                            <br /><br />
                            Estimated effort remaining: <strong>14 hours</strong>. 
                            <br />
                            Historical velocity: <strong>2.5 hours/day</strong>.
                            <br />
                            Statistical Completion Chance: <strong className="text-red-500">31%</strong>.
                          </p>
                        </div>
                        <span className="text-[9px] font-mono text-white/30 block mt-4 border-t border-white/10 pt-2 uppercase">
                          No fluff. Only mathematical facts.
                        </span>
                      </motion.div>

                      {/* Alert Card 2: Context Decay */}
                      <motion.div
                        initial={{ opacity: 0, scale: 0.98 }}
                        animate={{ opacity: 1, scale: 1 }}
                        transition={{ delay: 0.1 }}
                        className="bg-[#0F0F0F] border-2 border-[#FF4E00] p-6 flex flex-col justify-between h-80 relative"
                      >
                        <div className="absolute top-4 right-4 bg-[#FF4E00] text-black font-mono font-black text-[9px] px-2 py-0.5 uppercase tracking-widest">
                          DECAY WARNING
                        </div>
                        <div>
                          <span className="text-[9px] font-mono text-[#FF4E00] uppercase tracking-widest block font-bold">CHRONOS INTERVENE ALERT</span>
                          <h4 className="text-lg font-bold font-mono text-white uppercase mt-2">CONTEXT INTEGRITY DECAYED</h4>
                          <p className="text-xs text-white/70 mt-3 leading-relaxed font-mono bg-[#FF4E00]/5 p-3 border border-[#FF4E00]/20">
                            You have not touched this codebase in <strong className="text-white">{daysUntouched} days</strong>.
                            <br /><br />
                            Calculated Restart Cost: <strong>45 minutes</strong> of research tax.
                            <br /><br />
                            Context reconstruction checkpoint is available to reduce this to near zero.
                          </p>
                        </div>
                        <button 
                          onClick={() => setActiveMode('passive')}
                          className="bg-[#FF4E00] hover:bg-white text-black font-mono text-[10px] font-black uppercase py-2 text-center mt-3 cursor-pointer"
                        >
                          OPEN WORKSPACE RESTORER
                        </button>
                      </motion.div>

                      {/* Alert Card 3: Commitment Drift */}
                      <motion.div
                        initial={{ opacity: 0, scale: 0.98 }}
                        animate={{ opacity: 1, scale: 1 }}
                        transition={{ delay: 0.2 }}
                        className="bg-black border border-white/20 p-6 flex flex-col justify-between h-80 relative"
                      >
                        <div className="absolute top-4 right-4 border border-white/20 text-white/60 font-mono text-[9px] px-2 py-0.5 uppercase tracking-widest">
                          DRIFT NOTICE
                        </div>
                        <div>
                          <span className="text-[9px] font-mono text-white/40 uppercase tracking-widest block font-bold font-mono">CHRONOS INTERVENE ALERT</span>
                          <h4 className="text-lg font-bold font-mono text-white uppercase mt-2">COMMITMENT DRIFT IDENTIFIED</h4>
                          <p className="text-xs text-white/70 mt-3 leading-relaxed font-mono bg-white/5 p-3 border border-white/10">
                            You committed to: <br />
                            <strong className="text-[#FF4E00]">"Finish literature review webhook"</strong>.
                            <br /><br />
                            Sovereign filesystem tracking identified <strong>0 commits or file modifications</strong> matching this scope for <strong className="text-white">{commitmentDrift} days</strong>.
                          </p>
                        </div>
                        <span className="text-[9px] font-mono text-white/30 block mt-4 border-t border-white/10 pt-2 uppercase">
                          Verify commitment path.
                        </span>
                      </motion.div>

                    </div>
                  ) : (
                    <div className="border border-dashed border-white/10 p-12 text-center text-xs font-mono text-white/30">
                      CHRONOS IS CURRENTLY MUTED: COST OF INTERRUPTION ({interruptionCost}) EXCEEDS SILENCE DEBT ({silenceCost}).
                      <br /><br />
                      No intervention alerts are currently dispatched to preserve your active focus. Adjust sliders above to simulate alerts!
                    </div>
                  )}
                </div>
              </div>
            )}

          </motion.div>
        </AnimatePresence>

      </main>

      {/* Footer copyright */}
      <footer className="mt-16 border-t border-white/10 bg-[#070707] py-8 flex flex-col sm:flex-row items-center px-12 justify-between text-[9px] uppercase tracking-[0.2em] text-white/30 gap-4 font-mono">
        <div>© 2026 CHRONOS OPERATING SYSTEM GROUP / SOVEREIGN CORE</div>
        <div className="flex gap-10">
          <span>Privacy Sovereignty Guaranteed</span>
          <span className="text-[#FF4E00]">Stable Loopback Connection</span>
        </div>
      </footer>
    </div>
  );
}
