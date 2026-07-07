/**
 * @license
 * SPDX-License-Identifier: Apache-2.0
 */

import React, { useState, useEffect } from 'react';
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
  FileText
} from 'lucide-react';
import { motion, AnimatePresence } from 'motion/react';
import InteractiveSandbox from './components/InteractiveSandbox';
import RiskForecaster from './components/RiskForecaster';
import CommitmentList from './components/CommitmentList';
import WorkspaceRestorer from './components/WorkspaceRestorer';
import ARCPanel from './components/ARCPanel';
import DatabaseViewer from './components/DatabaseViewer';
import TelemetryConsole from './components/TelemetryConsole';
import DecisionVisibilityPanel from './components/DecisionVisibilityPanel';
import { computeCognitiveState } from './cognitive';
import { CognitiveState } from './cognitive/types';
import { Commitment, ProjectAction, ChronosEvent } from './types';

// redefined ViewMode based on the three fundamental modes of Chronos
type ViewMode = 'invisible' | 'passive' | 'intervention';

export default function App() {
  const [systemTime, setSystemTime] = useState<string>('2026-06-23T10:15:00-07:00');
  const [hasApiKey, setHasApiKey] = useState(true); // default true for isolated sovereign daemon
  const [commitments, setCommitments] = useState<Commitment[]>([]);
  const [actions, setActions] = useState<ProjectAction[]>([]);
  const [projects, setProjects] = useState<any[]>([]);
  const [refreshTrigger, setRefreshTrigger] = useState(0);
  const [activeMode, setActiveMode] = useState<ViewMode>('passive');
  const [eventHistory, setEventHistory] = useState<ChronosEvent[]>([]);
  const [timeOffsetHours, setTimeOffsetHours] = useState<number>(0);

  // Sliders for Mode 3: Intervention Simulator
  const [daysUntouched, setDaysUntouched] = useState<number>(11);
  const [deadlineHours, setDeadlineHours] = useState<number>(48);
  const [commitmentDrift, setCommitmentDrift] = useState<number>(5);
  const [focusIntensity, setFocusIntensity] = useState<number>(8);

  const [flightLogs, setFlightLogs] = useState<string[]>([]);
  const [simulatingLog, setSimulatingLog] = useState(false);

  // Telemetry Observability State
  const [lastSequenceNumber, setLastSequenceNumber] = useState<number | null>(null);
  const [latencyHistory, setLatencyHistory] = useState<any[]>([]);
  const [streamLagMs, setStreamLagMs] = useState<number>(0);
  const [droppedCount, setDroppedCount] = useState<number>(0);
  const [outOfOrderCount, setOutOfOrderCount] = useState<number>(0);
  const [slowTicksCount, setSlowTicksCount] = useState<number>(0);
  const [telemetryAlerts, setTelemetryAlerts] = useState<any[]>([]);
  const [cognitiveState, setCognitiveState] = useState<CognitiveState | null>(null);

  // Client-side reducer to construct state from the durable SQLite event log
  const rebuildState = (eventsList: ChronosEvent[], offset: number) => {
    const commitmentsMap = new Map<string, Commitment>();
    const actionsMap = new Map<string, ProjectAction>();
    const projectsMap = new Map<number, any>();
    const logs: string[] = [];
    let latestTime = '2026-06-23T10:15:00-07:00';

    // Seed default projects for visual structure
    projectsMap.set(1, {
      id: 1,
      project_name: 'ML Coursework Assignment',
      status: 'ACTIVE',
      created_at: '2026-06-09T10:00:00Z',
      state: {
        current_summary: 'Analyzing retrieval model and optimizing chunking strategies.',
        next_action: 'Investigate alternative chunking/splitting strategies.'
      },
      deadlines: [
        { id: 1, project_id: 1, deadline_label: 'ML Assignment Final Submission', target_date: '2026-07-15T23:59:59Z', importance_tier: 'HIGH' }
      ]
    });

    projectsMap.set(2, {
      id: 2,
      project_name: 'Stripe Billing Integration',
      status: 'ACTIVE',
      created_at: '2026-05-24T10:00:00Z',
      state: {
        current_summary: 'Implementing signature verification webhook.',
        next_action: 'Test webhooks locally using Stripe CLI sandbox.'
      },
      deadlines: [
        { id: 2, project_id: 2, deadline_label: 'Stripe Billing Production Deployment', target_date: '2026-06-25T18:00:00Z', importance_tier: 'CRITICAL' }
      ]
    });

    let projectCounter = 3;

    const processSingleEvent = (event: ChronosEvent) => {
      if (event.timestamp) {
        latestTime = event.timestamp;
      }

      if (event.event_type === 'GitRepositoryDiscovered') {
        const repoPath = event.payload.repository_path || '';
        const repoName = repoPath.split(/[\\/]/).pop() || 'Workspace';
        logs.push(`[FS_MONITOR] TOUCHED REPOSITORY: ${repoPath}`);
        
        let existing = Array.from(projectsMap.values()).find(p => p.project_name.toLowerCase() === repoName.toLowerCase());
        if (!existing) {
          projectsMap.set(projectCounter, {
            id: projectCounter,
            project_name: repoName,
            status: 'ACTIVE',
            created_at: event.timestamp,
            state: {
              current_summary: `Repository monitored at ${repoPath}`,
              next_action: 'Explore code modifications.'
            },
            deadlines: []
          });
          projectCounter++;
        }
      } else if (event.event_type === 'GitCommitCreated') {
        const msg = event.payload.source_payload?.message || '';
        logs.push(`[GIT_DAEMON] COMMIT DETECTED: ${msg}`);
      } else if (event.event_type === 'VSCodeActiveFileChanged' || event.event_type === 'FileModified') {
        const path = event.payload.file_path || event.payload.path || '';
        logs.push(`[FS_MONITOR] TOUCHED FILE: ${path}`);
      } else if (event.event_type === 'BrowserTabFocused' || event.event_type === 'BrowserTabActivated') {
        const title = event.payload.title || '';
        logs.push(`[BROWSER_MONITOR] ACTIVE TAB DETECTED: ${title}`);
      } else if (event.event_type === 'CommandLineExecuted' || event.event_type === 'EditorTerminalCommandRun') {
        const cmd = event.payload.command || event.payload.display_name || '';
        logs.push(`[SHELL_MONITOR] PROCESS STARTED: ${cmd}`);
      } else if (event.event_type === 'CognitiveConflictDetected') {
        const desc = event.payload.details || '';
        logs.push(`[COGNITIVE_TRACE] COGNITIVE DRIFT DETECTED: ${desc}`);
      } else if (event.event_type === 'CommitmentDiscovered') {
        const payload = event.payload;
        const title = payload.content || '';
        let p_id = 1;
        if (title.toLowerCase().includes('stripe') || title.toLowerCase().includes('webhook') || title.toLowerCase().includes('billing')) {
          p_id = 2;
        }
        commitmentsMap.set(payload.commitment_id, {
          id: commitmentsMap.size + 1,
          project_id: p_id,
          title: title,
          commitment_type: (payload.source === 'Manual' ? 'OBLIGATION' : 'ASSIGNMENT'),
          deadline_date: payload.inferred_due_at || undefined,
          confidence_score: payload.confidence || 0.9,
          source_node_id: null,
          status: 'OPEN',
          created_at: payload.created_at || event.timestamp,
          health: 'GREEN',
          risk_score: 0.1,
          completion_chance: 0.95
        });
      } else if (event.event_type === 'CommitmentUpdated') {
        const payload = event.payload;
        const c = commitmentsMap.get(payload.commitment_id);
        if (c) {
          if (payload.confidence) c.confidence_score = payload.confidence;
        }
      } else if (event.event_type === 'CommitmentMarkedAtRisk') {
        const c = commitmentsMap.get(event.payload.commitment_id);
        if (c) {
          c.status = 'OPEN';
          c.health = 'RED';
        }
      } else if (event.event_type === 'CommitmentCompleted') {
        const c = commitmentsMap.get(event.payload.commitment_id);
        if (c) {
          c.status = 'COMPLETED';
          c.health = 'GREEN';
        }
      } else if (event.event_type === 'CommitmentCancelled') {
        const c = commitmentsMap.get(event.payload.commitment_id);
        if (c) {
          c.status = 'ABANDONED';
        }
      } else if (event.event_type === 'ExecutionPlanCreated') {
        const payload = event.payload;
        const plan = payload.plan;
        actionsMap.set(plan.execution_plan_id, {
          id: actionsMap.size + 1,
          project_id: 1,
          action_text: `Execute plan: ${plan.execution_steps.join(' -> ')}`,
          estimated_effort_hours: 1.0,
          status: 'PENDING',
          priority_score: 8.5,
          created_at: event.timestamp
        });
      }
    };

    // Flatten TickFrames and normal events
    for (const ev of eventsList) {
      if (ev.event_type === 'TickFrameEmitted') {
        const frame = ev.payload;
        if (frame) {
          if (frame.tick_completed_at) latestTime = frame.tick_completed_at;
          else if (frame.tick_started_at) latestTime = frame.tick_started_at;
          if (frame.perception) frame.perception.forEach(processSingleEvent);
          if (frame.reasoning) frame.reasoning.forEach(processSingleEvent);
          if (frame.decision) frame.decision.forEach(processSingleEvent);
          if (frame.execution) frame.execution.forEach(processSingleEvent);
          if (frame.feedback) frame.feedback.forEach(processSingleEvent);
        }
      } else {
        processSingleEvent(ev);
      }
    }

    const timeNow = new Date(new Date(latestTime).getTime() + offset * 60 * 60 * 1000);
    const commitmentsArray = Array.from(commitmentsMap.values()).map(c => {
      const projectActions = Array.from(actionsMap.values()).filter(a => a.project_id === c.project_id);
      const completedActions = projectActions.filter(a => a.status === 'COMPLETED').length;
      const totalActions = projectActions.length;
      const prog = totalActions > 0 ? (completedActions / totalActions) : 1.0;

      let t_norm = 1.0;
      let daysRemaining = 7;
      if (c.deadline_date) {
        const timeRemainingMs = new Date(c.deadline_date).getTime() - timeNow.getTime();
        const timeRemainingHours = Math.max(0, timeRemainingMs / (60 * 60 * 1000));
        daysRemaining = Math.max(0.1, timeRemainingHours / 24);
        t_norm = Math.min(1.0, timeRemainingHours / 168);
      }

      const estimatedRemainingHours = projectActions.filter(a => a.status === 'PENDING').reduce((sum, a) => sum + a.estimated_effort_hours, 0);
      const e_norm = Math.min(1.0, estimatedRemainingHours / 168);

      const w_prog = 0.30;
      const w_act = 0.25;
      const w_time = 0.25;
      const w_eff = 0.20;

      const act_rec = 0.8;
      const healthVal = (w_prog * prog) + (w_act * act_rec) + (w_time * t_norm) - (w_eff * e_norm);

      let health: 'GREEN' | 'YELLOW' | 'RED' = 'GREEN';
      if (healthVal < 0.40) health = 'RED';
      else if (healthVal < 0.70) health = 'YELLOW';

      const dailyCapacityHours = 2.0;
      const availableHoursWindow = daysRemaining * dailyCapacityHours;
      const riskScore = parseFloat(Math.min(2.0, estimatedRemainingHours / Math.max(0.1, availableHoursWindow)).toFixed(2));
      const completionChance = parseFloat((1 / (1 + Math.exp(6 * (riskScore - 1.0)))).toFixed(2));

      return {
        ...c,
        health: c.health === 'RED' ? 'RED' : health,
        risk_score: riskScore,
        completion_chance: completionChance
      };
    });

    return {
      projects: Array.from(projectsMap.values()),
      commitments: commitmentsArray,
      actions: Array.from(actionsMap.values()),
      flightLogs: logs.length > 0 ? logs : [
        '[INIT] Establishing context security layer - 100% isolated local SQLite database',
        '[FS_MONITOR] Scanning workspace folders for cognitive focus indicators...',
        '[DEAMON] SILENT CONTEXT PERSISTENCE SECURED. NO USER DISTRACTION EVENT PERMITTED.'
      ],
      systemTime: timeNow.toISOString()
    };
  };

  const handleSimulateLog = () => {
    setSimulatingLog(true);
    const possibleActions = [
      '[FS_MONITOR] TOUCHED FILE: src/billing/invoice_model.ts (L: 112, C: 4)',
      '[BROWSER_MONITOR] SWITCHED TAB: stripe.com/docs/api/invoice_creation',
      '[SHELL_MONITOR] PROCESS SHUTDOWN: tsc --watch (Exited with code 0)',
      '[GIT_DAEMON] COMMIT DETECTED: f42a1b9 "WIP: invoice rate checks"',
      '[COGNITIVE_TRACE] BLOCKAGE RESOLVED: User moved past Stripe header verification',
      '[IDLE_CRAWLER] Background Crawler indexing database transaction logs...'
    ];
    const randomAction = possibleActions[Math.floor(Math.random() * possibleActions.length)];
    setTimeout(() => {
      setFlightLogs(prev => [...prev, `[${new Date().toLocaleTimeString()}] ${randomAction}`]);
      setSimulatingLog(false);
    }, 450);
  };

  const fetchData = async () => {
    try {
      // Fetch historical events stream from the real SQLite database via the Axum bridge
      const eventsRes = await fetch('/api/events/stream');
      const data = await eventsRes.json();
      const eventsList: ChronosEvent[] = data.events || [];
      
      setEventHistory(eventsList);

      const state = rebuildState(eventsList, timeOffsetHours);
      setProjects(state.projects);
      setCommitments(state.commitments);
      setActions(state.actions);
      setFlightLogs(state.flightLogs);
      setSystemTime(state.systemTime);

      const cognState = computeCognitiveState(eventsList, streamLagMs, droppedCount, latencyHistory);
      setCognitiveState(cognState);
    } catch (e) {
      console.error('Error fetching Chronos events:', e);
    }
  };

  useEffect(() => {
    fetchData();
  }, [refreshTrigger, timeOffsetHours]);

  // Real‑time Chronos Event Stream via Server‑Sent Events (SSE)
  useEffect(() => {
    const eventSource = new EventSource('/api/events/stream/live');
    eventSource.onmessage = (e) => {
      try {
        const receivedTime = new Date();
        const ev = JSON.parse(e.data) as ChronosEvent;
        setEventHistory((prev) => {
          const newHist = [...prev, ev];
          const deduplicated = newHist.filter((item, idx) => newHist.findIndex(x => x.id === item.id) === idx);
          
          // Re-render state reactively from the updated history stream
          const state = rebuildState(deduplicated, timeOffsetHours);
          setProjects(state.projects);
          setCommitments(state.commitments);
          setActions(state.actions);
          setFlightLogs(state.flightLogs);
          setSystemTime(state.systemTime);

          const cognState = computeCognitiveState(deduplicated, streamLagMs, droppedCount, latencyHistory);
          setCognitiveState(cognState);

          // Real-time telemetry monitoring for TickFrameEmitted events
          if (ev.event_type === 'TickFrameEmitted') {
            const frame = ev.payload;
            const parsedTime = new Date();
            if (frame && frame.telemetry) {
              const telemetry = frame.telemetry;

              // 1. Check for Out-of-order & Dropped Frames
              if (lastSequenceNumber !== null) {
                if (telemetry.tick_sequence <= lastSequenceNumber) {
                  setOutOfOrderCount(c => c + 1);
                  setTelemetryAlerts(prev => [...prev, {
                    id: Math.random().toString(),
                    timestamp: new Date().toLocaleTimeString(),
                    severity: 'WARN',
                    message: `Out-of-order frame: seq ${telemetry.tick_sequence} (expected > ${lastSequenceNumber})`,
                    tick_id: frame.tick_id
                  }]);
                  setTimeout(fetchData, 100);
                } else if (telemetry.tick_sequence > lastSequenceNumber + 1) {
                  setDroppedCount(c => c + 1);
                  setTelemetryAlerts(prev => [...prev, {
                    id: Math.random().toString(),
                    timestamp: new Date().toLocaleTimeString(),
                    severity: 'WARN',
                    message: `Dropped frame: seq ${telemetry.tick_sequence} (expected ${lastSequenceNumber + 1})`,
                    tick_id: frame.tick_id
                  }]);
                  setTimeout(fetchData, 100);
                }
              }
              setLastSequenceNumber(telemetry.tick_sequence);

              // 2. Measure delivery lag
              const completedTime = telemetry.tick_execution_completed_time 
                ? new Date(telemetry.tick_execution_completed_time) 
                : new Date(frame.tick_completed_at || frame.tick_started_at);
              const deliveryLag = Math.max(0, receivedTime.getTime() - completedTime.getTime());
              setStreamLagMs(deliveryLag);
              if (deliveryLag > 100) {
                setTelemetryAlerts(prev => [...prev, {
                  id: Math.random().toString(),
                  timestamp: new Date().toLocaleTimeString(),
                  severity: 'WARN',
                  message: `SSE Delivery lag detected: ${deliveryLag}ms`,
                  tick_id: frame.tick_id
                }]);
              }

              // Slow tick detection (> 50ms cap)
              if (telemetry.total_duration_ms > 50) {
                setSlowTicksCount(c => c + 1);
                setTelemetryAlerts(prev => [...prev, {
                  id: Math.random().toString(),
                  timestamp: new Date().toLocaleTimeString(),
                  severity: 'WARN',
                  message: `Slow tick: total duration ${telemetry.total_duration_ms}ms (exceeds 50ms target)`,
                  tick_id: frame.tick_id
                }]);
              }

              // Ingest backend anomalies from reasoning & execution vectors
              if (frame.reasoning) {
                frame.reasoning.forEach((ev: any) => {
                  if (ev.event_type === 'TickPerformanceWarning') {
                    setTelemetryAlerts(prev => {
                      if (prev.some(a => a.id === ev.id)) return prev;
                      return [...prev, {
                        id: ev.id,
                        timestamp: new Date(ev.timestamp).toLocaleTimeString(),
                        severity: 'WARN',
                        message: ev.payload.warning || 'Slow tick warning',
                        tick_id: frame.tick_id
                      }];
                    });
                  }
                });
              }

              if (frame.execution) {
                frame.execution.forEach((ev: any) => {
                  if (ev.event_type === 'ExecutionFailed' || ev.event_type === 'ActionFailed') {
                    setTelemetryAlerts(prev => {
                      if (prev.some(a => a.id === ev.id)) return prev;
                      return [...prev, {
                        id: ev.id,
                        timestamp: new Date(ev.timestamp).toLocaleTimeString(),
                        severity: 'CRITICAL',
                        message: ev.payload.error || 'Execution phase failed',
                        tick_id: frame.tick_id
                      }];
                    });
                  }
                });
              }

              // 3. UI Render Confirmation ACK loop
              const startRender = performance.now();
              setTimeout(async () => {
                const renderEnd = performance.now();
                const uiRenderTime = Math.round(renderEnd - startRender);
                const renderedTime = new Date();

                const IngestedTime = telemetry.perception_ingestion_time 
                  ? new Date(telemetry.perception_ingestion_time) 
                  : new Date(telemetry.tick_execution_start_time);
                const startTick = new Date(telemetry.tick_execution_start_time);
                const perceptionToStart = Math.max(0, startTick.getTime() - IngestedTime.getTime());

                const breakdown = {
                  perception_to_execution_start_ms: perceptionToStart,
                  tick_processing_ms: telemetry.total_duration_ms,
                  network_delivery_lag_ms: deliveryLag,
                  ui_render_time_ms: uiRenderTime
                };

                setLatencyHistory(prev => {
                  const updated = [...prev, { tick_id: frame.tick_id, sequence: telemetry.tick_sequence, ...breakdown }];
                  return updated.slice(-10);
                });

                try {
                  await fetch('/api/telemetry/ack', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({
                      tick_id: frame.tick_id,
                      received_at: receivedTime.toISOString(),
                      parsed_at: parsedTime.toISOString(),
                      rendered_at: renderedTime.toISOString(),
                      latency_breakdown: breakdown
                    })
                  });
                } catch (ackError) {
                  console.error('Failed to post UI telemetry ACK', ackError);
                }
              }, 0);
            }
          }

          return deduplicated;
        });
      } catch (err) {
        console.error('Failed to parse ChronosEvent', err);
      }
    };
    eventSource.onerror = (err) => {
      console.error('SSE connection error', err);
      eventSource.close();
    };
    return () => {
      eventSource.close();
    };
  }, [timeOffsetHours]);

  const handleRefresh = () => {
    setRefreshTrigger(prev => prev + 1);
  };

  const handleTimeTravel = async (hours: number) => {
    // Advance simulated clock locally to update risk curves and attention decays
    setTimeOffsetHours(prev => prev + hours);
  };

  const handleToggleAction = async (actionId: number) => {
    // In live execution, actions represent tasks generated by DecisionEngine/Cce.
    // Completing an action is represented by setting its status locally.
    setActions(prev => prev.map(a => a.id === actionId ? { ...a, status: a.status === 'COMPLETED' ? 'PENDING' : 'COMPLETED' } : a));
  };

  // Math variables for the Decision Engine Simulator
  const silenceCost = Number((daysUntouched * 4.5 + commitmentDrift * 8.0 + (120 - deadlineHours) * 0.6).toFixed(1));
  const interruptionCost = Number((focusIntensity * 2.5 + 12).toFixed(1));
  const shouldIntervene = silenceCost > interruptionCost;

  const anyRedCommitment = commitments.some(c => c.health === 'RED');

  return (
    <div className="min-h-screen bg-[#070707] text-[#F4F4F1] pb-16 select-none antialiased font-sans">
        {/* Live Event Stream */}
        <div className="my-4 p-4 bg-[#111] rounded">
          <h2 className="text-lg font-bold mb-2">Live Chronos Events</h2>
          <ul className="max-h-64 overflow-y-auto text-sm">
            {eventHistory.slice(-20).map((ev) => (
              <li key={ev.id}>
                <span className="text-gray-400">{new Date(ev.timestamp).toLocaleTimeString()}</span> - {ev.event_type}
              </li>
            ))}
          </ul>
        </div>
      
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
              <div className="space-y-6">
                
                {/* Visual Status card */}
                <div className="bg-[#0F0F0F] border border-white/10 p-6 flex flex-col md:flex-row items-center justify-between gap-6 relative overflow-hidden">
                  <div className="absolute right-0 top-0 w-48 h-48 bg-gradient-to-br from-[#FF4E00]/5 to-transparent pointer-events-none" />
                  <div className="space-y-2">
                    <div className="flex items-center gap-2">
                      <div className="w-2.5 h-2.5 bg-green-500 rounded-full animate-pulse" />
                      <span className="text-xs font-mono font-bold uppercase tracking-widest text-[#FF4E00]">
                        FLIGHT RECORDER DAEMON ACTIVE
                      </span>
                    </div>
                    <h2 className="text-2xl font-black italic uppercase text-white tracking-tight">
                      BLACK BOX STREAM
                    </h2>
                    <p className="text-xs text-white/50 max-w-xl leading-relaxed">
                      While you work, Chronos remains entirely invisible. It maps folders, records browser sessions, and builds semantic graphs without interrupting your focus flow.
                    </p>
                  </div>
                  <div className="bg-black p-4 border border-white/5 font-mono text-[10px] space-y-1 rounded-none shrink-0 w-full md:w-auto">
                    <div className="text-white/40 uppercase tracking-widest border-b border-white/5 pb-1.5 mb-1.5">DAEMON STATISTICS</div>
                    <div className="flex justify-between gap-10"><span>INTERRUPTION COST:</span><span className="text-white">{interruptionCost} UNITS</span></div>
                    <div className="flex justify-between gap-10"><span>SILENCE COST:</span><span className="text-[#FF4E00] font-bold">{silenceCost} UNITS</span></div>
                    <div className="flex justify-between gap-10"><span>STATUS:</span><span className="text-green-400 font-bold">STAY_INVISIBLE</span></div>
                  </div>
                </div>

                {/* Live Terminal Console Log */}
                <div className="bg-black border border-white/10 rounded-none p-5 font-mono text-xs">
                  <div className="flex items-center justify-between border-b border-white/10 pb-3 mb-4">
                    <div className="flex items-center gap-2 text-white/80">
                      <Terminal className="w-4 h-4 text-[#FF4E00]" />
                      <span>TELEMETRY_FLIGHT_LOGS.log</span>
                    </div>
                    <button
                      onClick={handleSimulateLog}
                      disabled={simulatingLog}
                      className="text-[10px] font-bold uppercase tracking-wider bg-[#FF4E00] hover:bg-white text-black py-1.5 px-3.5 transition disabled:opacity-50 cursor-pointer"
                    >
                      {simulatingLog ? 'CAPTURING...' : 'SIMULATE FILE ACCESS / TAB CHANGE'}
                    </button>
                  </div>

                  <div className="space-y-2.5 max-h-[420px] overflow-y-auto pr-2 text-white/70">
                    {flightLogs.map((log, idx) => {
                      let color = 'text-white/50';
                      if (log.includes('[FS_MONITOR]')) color = 'text-blue-400';
                      if (log.includes('[BROWSER_MONITOR]')) color = 'text-purple-400';
                      if (log.includes('[ARC_RESEARCH]')) color = 'text-yellow-500';
                      if (log.includes('[COGNITIVE_TRACE]')) color = 'text-[#FF4E00]';
                      if (log.includes('[INIT]')) color = 'text-green-400 font-bold';
                      return (
                        <div key={idx} className="flex items-start gap-3 border-b border-white/5 pb-2">
                          <span className="text-white/20 select-none">[{String(idx + 1).padStart(2, '0')}]</span>
                          <span className={`${color} leading-relaxed break-all`}>{log}</span>
                        </div>
                      );
                    })}
                  </div>
                </div>

                {/* Interactive Simulator sandbox trigger */}
                <div className="bg-[#0F0F0F] border border-white/5 p-5">
                  <span className="text-[10px] font-mono uppercase tracking-[0.2em] text-white/40 block mb-3">Time & Checkpoint Event Simulation Sandbox</span>
                  <InteractiveSandbox
                    systemTime={systemTime}
                    hasApiKey={hasApiKey}
                    onRefresh={handleRefresh}
                    onTimeTravel={handleTimeTravel}
                  />
                </div>
              </div>
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
                        <p className="text-xs text-white/80 font-mono leading-relaxed bg-[#111] p-3 border border-white/5">
                          {(() => {
                            const conflictEvent = [...eventHistory].reverse().find(e => e.event_type === 'CognitiveConflictDetected');
                            if (conflictEvent && conflictEvent.payload && conflictEvent.payload.details) {
                              return conflictEvent.payload.details;
                            }
                            return "No active blockages. Development momentum is healthy.";
                          })()}
                        </p>
                      </div>
                      
                      {/* Integrated ARC background crawlers panel */}
                      <div className="border border-white/5 p-1 bg-black">
                        <ARCPanel events={eventHistory} />
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
                      <WorkspaceRestorer 
                        projects={projects}
                        onRefresh={handleRefresh}
                      />
                    </div>
                  </div>

                </div>

                {/* Sovereignty verification database audit tool */}
                <div className="pt-6 border-t border-white/10">
                  <DatabaseViewer
                    systemTime={systemTime}
                    refreshTrigger={refreshTrigger}
                    onRefresh={handleRefresh}
                    events={eventHistory}
                  />
                </div>

                {/* Telemetry Observability & Tracing Console */}
                <TelemetryConsole
                  tickSequence={lastSequenceNumber}
                  streamLagMs={streamLagMs}
                  droppedCount={droppedCount}
                  outOfOrderCount={outOfOrderCount}
                  slowTicksCount={slowTicksCount}
                  alerts={telemetryAlerts}
                  latencyHistory={latencyHistory}
                  cognitiveState={cognitiveState}
                />

                {/* Cognitive Semantic Layer (CSL) Visibility Panel */}
                <DecisionVisibilityPanel cognitiveState={cognitiveState} />
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
