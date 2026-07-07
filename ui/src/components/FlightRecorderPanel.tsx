/**
 * @license
 * SPDX-License-Identifier: Apache-2.0
 */

import React, { useState, useEffect, useMemo } from 'react';
import { 
  Terminal, 
  Cpu, 
  FileCode, 
  Globe, 
  Search, 
  CheckSquare, 
  Clock, 
  Play, 
  Database, 
  Eye, 
  ArrowUpRight, 
  Sliders, 
  Download, 
  Activity, 
  X, 
  Info,
  Calendar,
  AlertTriangle,
  FolderSync,
  Trash2,
  MessageSquare,
  Plus
} from 'lucide-react';
import { motion, AnimatePresence } from 'motion/react';
import { TelemetryLog } from '../types';
import { API_BASE } from '../config';

interface FlightRecorderPanelProps {
  systemTime: string;
  hasApiKey: boolean;
  flightLogs: TelemetryLog[];
  simulatingLog: boolean;
  onRefresh: () => void;
  onTimeTravel: (hours: number) => void;
}

function formatRelativeTimestamp(rawTs: string, fallbackTs: string, sysTime: string): string {
  if (!rawTs) return fallbackTs;
  
  try {
    const logDate = new Date(rawTs);
    const sysDate = new Date(sysTime);
    
    if (isNaN(logDate.getTime()) || isNaN(sysDate.getTime())) {
      return fallbackTs;
    }
    
    // Format the time as HH:MM:SS local time
    const timeStr = logDate.toLocaleTimeString(undefined, {
      hour12: false,
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit'
    });

    const diffTime = sysDate.getTime() - logDate.getTime();
    
    // If difference is negative, it's in the future (due to simulation or sync)
    if (diffTime < 0) {
      const dateStr = logDate.toLocaleDateString(undefined, {
        month: 'short',
        day: 'numeric'
      });
      return `${dateStr}, ${timeStr}`;
    }

    const diffDays = Math.floor(diffTime / (1000 * 60 * 60 * 24));
    
    // Check if same calendar day
    const isSameDay = logDate.getFullYear() === sysDate.getFullYear() &&
                      logDate.getMonth() === sysDate.getMonth() &&
                      logDate.getDate() === sysDate.getDate();

    // Check if calendar yesterday
    const yesterday = new Date(sysDate);
    yesterday.setDate(sysDate.getDate() - 1);
    const isYesterday = logDate.getFullYear() === yesterday.getFullYear() &&
                        logDate.getMonth() === yesterday.getMonth() &&
                        logDate.getDate() === yesterday.getDate();

    if (isSameDay) {
      return `Today, ${timeStr}`;
    } else if (isYesterday) {
      return `Yesterday, ${timeStr}`;
    } else if (diffDays < 7) {
      const actualDiff = diffDays === 0 ? 1 : diffDays;
      return `${actualDiff} day${actualDiff > 1 ? 's' : ''} ago, ${timeStr}`;
    } else {
      const dateStr = logDate.toLocaleDateString(undefined, {
        month: 'short',
        day: 'numeric'
      });
      return `${dateStr}, ${timeStr}`;
    }
  } catch (e) {
    return fallbackTs;
  }
}

export default function FlightRecorderPanel({
  systemTime,
  hasApiKey,
  flightLogs,
  simulatingLog,
  onRefresh,
  onTimeTravel
}: FlightRecorderPanelProps) {
  const [filterCategory, setFilterCategory] = useState<string>('ALL');
  const [searchQuery, setSearchQuery] = useState<string>('');
  const [selectedLog, setSelectedLog] = useState<TelemetryLog | null>(null);
  const [activeTab, setActiveTab] = useState<'timeline' | 'insights'>('timeline');
  const [deletingId, setDeletingId] = useState<number | null>(null);
  const [expandedGroups, setExpandedGroups] = useState<Record<string, boolean>>({});

  const [showManualIngest, setShowManualIngest] = useState<boolean>(false);
  const [manualTitle, setManualTitle] = useState<string>('');
  const [manualDetail, setManualDetail] = useState<string>('');
  const [manualType, setManualType] = useState<'URL' | 'FILE' | 'TEXT'>('URL');
  const [ingesting, setIngesting] = useState<boolean>(false);
  const [ingestError, setIngestError] = useState<string | null>(null);

  const handleManualIngest = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!manualTitle.trim() || !manualDetail.trim()) {
      setIngestError('Please fill in both the title and the details.');
      return;
    }
    setIngesting(true);
    setIngestError(null);
    try {
      const response = await fetch(`${API_BASE}/api/telemetry/ingest`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          displayName: manualTitle,
          detail: manualDetail,
          entityType: manualType,
        }),
      });
      if (response.ok) {
        setManualTitle('');
        setManualDetail('');
        setShowManualIngest(false);
        onRefresh();
      } else {
        const text = await response.text();
        setIngestError(`Ingestion failed: ${text || response.statusText}`);
      }
    } catch (err) {
      console.error('Error during manual ingestion:', err);
      setIngestError('Network error. Failed to send manual ingestion.');
    } finally {
      setIngesting(false);
    }
  };

  const getLogKey = (log: TelemetryLog) => {
    if (log.ids && log.ids.length > 0) return `group-${log.ids.join('-')}`;
    if (log.id !== undefined && log.id !== -1) return `log-${log.id}`;
    return `sys-${log.display_name.replace(/\s+/g, '')}`;
  };

  const toggleGroup = (groupKey: string) => {
    setExpandedGroups(prev => ({
      ...prev,
      [groupKey]: !prev[groupKey]
    }));
  };

  const handleRemoveLog = async (logId: number | null, eventType: string, logIds?: number[]) => {
    const displayId = logId || (logIds && logIds[0]) || 0;
    setDeletingId(displayId);
    try {
      const response = await fetch(`${API_BASE}/api/telemetry-logs`, {
        method: 'DELETE',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({ 
          id: logId, 
          ids: logIds, 
          event_type: eventType 
        }),
      });
      if (response.ok) {
        onRefresh();
      } else {
        console.error('Failed to delete telemetry log:', response.statusText);
      }
    } catch (err) {
      console.error('Error deleting telemetry log:', err);
    } finally {
      setDeletingId(null);
    }
  };

  // Event counts for badge stats
  const stats = useMemo(() => {
    const counts: Record<string, number> = {
      ALL: flightLogs.length,
      FS_MONITOR: 0,
      BROWSER_MONITOR: 0,
      TERMINAL_MONITOR: 0,
      IDE_MONITOR: 0,
      COMMUNICATION_MONITOR: 0,
      APP_MONITOR: 0,
      COGNITIVE_TRACE: 0,
      RECOVERY_PLAN: 0,
      SYSTEM: 0
    };
    flightLogs.forEach(log => {
      if (counts.hasOwnProperty(log.category)) {
        counts[log.category]++;
      }
    });
    return counts;
  }, [flightLogs]);

  // Filtered logs
  const filteredLogs = useMemo(() => {
    return flightLogs.filter(log => {
      let matchesCategory = false;
      if (filterCategory === 'ALL') {
        matchesCategory = true;
      } else if (log.category === filterCategory) {
        matchesCategory = true;
      } else if (log.event_type === 'COMPACTED') {
        const detailLower = log.detail.toLowerCase();
        if (filterCategory === 'FS_MONITOR' && (detailLower.includes('files worked on') || detailLower.includes('files'))) {
          matchesCategory = true;
        } else if (filterCategory === 'BROWSER_MONITOR' && (detailLower.includes('web resources researched') || detailLower.includes('research'))) {
          matchesCategory = true;
        } else if (filterCategory === 'TERMINAL_MONITOR') {
          const hasApps = detailLower.includes('applications & tools used') || detailLower.includes('applications') || detailLower.includes('tools') || detailLower.includes('apps');
          if (hasApps) {
            const terminalKeywords = ['terminal', 'cmd', 'command prompt', 'powershell', 'wt', 'bash', 'zsh', 'git-bash', 'git bash', 'conhost', 'alacritty', 'kitty'];
            if (terminalKeywords.some(keyword => detailLower.includes(keyword))) {
              matchesCategory = true;
            }
          }
        } else if (filterCategory === 'IDE_MONITOR') {
          const hasApps = detailLower.includes('applications & tools used') || detailLower.includes('applications') || detailLower.includes('tools') || detailLower.includes('apps');
          if (hasApps) {
            const ideKeywords = ['code', 'vscode', 'visual studio code', 'cursor', 'sublime', 'intellij', 'pycharm', 'webstorm', 'clion', 'rider', 'studio', 'eclipse', 'notepad++', 'vim', 'neovim', 'emacs'];
            if (ideKeywords.some(keyword => detailLower.includes(keyword))) {
              matchesCategory = true;
            }
          }
        } else if (filterCategory === 'COMMUNICATION_MONITOR') {
          const hasApps = detailLower.includes('applications & tools used') || detailLower.includes('applications') || detailLower.includes('tools') || detailLower.includes('apps');
          if (hasApps) {
            const commKeywords = ['whatsapp', 'slack', 'discord', 'teams', 'telegram', 'signal', 'messenger', 'skype', 'zoom'];
            if (commKeywords.some(keyword => detailLower.includes(keyword))) {
              matchesCategory = true;
            }
          }
        } else if (filterCategory === 'APP_MONITOR') {
          const appHeaderIdx = ['applications & tools used', 'applications', 'tools', 'apps']
            .map(h => detailLower.indexOf(h))
            .find(idx => idx !== -1);
          if (appHeaderIdx !== undefined && appHeaderIdx !== -1) {
            const allSpecialKeywords = [
              'terminal', 'cmd', 'command prompt', 'powershell', 'wt', 'bash', 'zsh', 'git-bash', 'git bash', 'conhost', 'alacritty', 'kitty',
              'code', 'vscode', 'visual studio code', 'cursor', 'sublime', 'intellij', 'pycharm', 'webstorm', 'clion', 'rider', 'studio', 'eclipse', 'notepad++', 'vim', 'neovim', 'emacs',
              'whatsapp', 'slack', 'discord', 'teams', 'telegram', 'signal', 'messenger', 'skype', 'zoom'
            ];
            const appSection = detailLower.substring(appHeaderIdx);
            const lines = appSection.split('\n');
            let foundOtherApp = false;
            for (const line of lines) {
              if (line.includes('consolidated') || line.includes('total context items')) break;
              if (line.trim().startsWith('* `') || line.trim().startsWith('*`')) {
                const match = line.match(/`([^`]+)`/);
                if (match) {
                  const appName = match[1];
                  const isSpecial = allSpecialKeywords.some(kw => appName.includes(kw));
                  if (!isSpecial) {
                    foundOtherApp = true;
                    break;
                  }
                }
              }
            }
            if (foundOtherApp) {
              matchesCategory = true;
            }
          }
        }
      }

      const matchesSearch = searchQuery.trim() === '' || 
        log.display_name.toLowerCase().includes(searchQuery.toLowerCase()) ||
        log.detail.toLowerCase().includes(searchQuery.toLowerCase()) ||
        log.event_type.toLowerCase().includes(searchQuery.toLowerCase());
      return matchesCategory && matchesSearch;
    });
  }, [flightLogs, filterCategory, searchQuery]);

  // Activity frequency pulses for interactive chart
  const activityPulses = useMemo(() => {
    return flightLogs.slice(0, 12).map((log, index) => {
      let height = 30;
      if (log.category === 'FS_MONITOR') height = 75;
      if (log.category === 'BROWSER_MONITOR') height = 50;
      if (log.category === 'TERMINAL_MONITOR') height = 80;
      if (log.category === 'IDE_MONITOR') height = 70;
      if (log.category === 'COMMUNICATION_MONITOR') height = 60;
      if (log.category === 'APP_MONITOR') height = 45;
      if (log.category === 'COGNITIVE_TRACE') height = 90;
      if (log.category === 'RECOVERY_PLAN') height = 100;
      return {
        id: index,
        category: log.category,
        height,
        time: formatRelativeTimestamp(log.raw_timestamp, log.timestamp, systemTime)
      };
    }).reverse();
  }, [flightLogs]);

  // Compute stats like "suppressed notifications"
  const interruptionCostSaved = useMemo(() => {
    // Assume each file edit & workspace tab switch would have been a notification or administrative distraction.
    // Every context node or event represents an interruption saved.
    const eligibleEvents = flightLogs.filter(log => 
      log.category === 'FS_MONITOR' || 
      log.category === 'BROWSER_MONITOR' ||
      log.category === 'TERMINAL_MONITOR' ||
      log.category === 'IDE_MONITOR' ||
      log.category === 'COMMUNICATION_MONITOR' ||
      log.category === 'APP_MONITOR'
    ).length;
    return eligibleEvents * 3; // 3 mins saved per interruption
  }, [flightLogs]);

  return (
    <div className="space-y-6">
      
      {/* 1. STUNNING STATUS CARD AND METRICS GRID */}
      <div className="relative overflow-hidden border border-white/10 bg-[#0C0C0C]/90 p-6 md:p-8 backdrop-blur-xl">
        <div className="absolute right-0 top-0 -mr-16 -mt-16 h-48 w-48 rounded-full bg-gradient-to-br from-[#FF4E00]/10 to-transparent blur-2xl pointer-events-none" />
        <div className="absolute left-0 bottom-0 h-1/2 w-full bg-gradient-to-t from-black/60 to-transparent pointer-events-none" />
        
        <div className="relative flex flex-col lg:flex-row lg:items-center justify-between gap-6">
          <div className="space-y-3">
            <div className="flex items-center gap-2">
              <span className="relative flex h-3 w-3">
                <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-[#FF4E00] opacity-75"></span>
                <span className="relative inline-flex rounded-full h-3 w-3 bg-[#FF4E00]"></span>
              </span>
              <span className="text-[10px] font-bold uppercase tracking-widest text-[#FF4E00] font-mono">
                FLIGHT RECORDER DAEMON RUNNING
              </span>
              <span className="text-[10px] px-2 py-0.5 bg-neutral-900 border border-white/10 text-white/50 rounded font-mono">
                SILENT_MODE
              </span>
            </div>
            
            <h1 className="text-3xl font-black tracking-tight text-white uppercase italic font-sans">
              Invisible Telemetry Black Box
            </h1>
            <p className="text-sm text-white/60 max-w-2xl leading-relaxed">
              Chronos intercepts developer workspace actions in the background. It reads active file paths, detects tab focuses, registers search queries, and aggregates cognitive context graphs without prompts, modals, or manual log checkins.
            </p>
          </div>

          {/* Quick Metrics */}
          <div className="grid grid-cols-2 sm:grid-cols-3 lg:flex lg:flex-row gap-4 shrink-0 w-full lg:w-auto">
            <div className="bg-black/60 border border-white/5 p-4 rounded-none min-w-[140px] flex flex-col justify-between">
              <span className="text-[9px] text-white/40 uppercase font-mono tracking-widest">Saved Focus</span>
              <div className="mt-2 flex items-baseline gap-1">
                <span className="text-2xl font-black text-white font-mono">{interruptionCostSaved}</span>
                <span className="text-[10px] text-[#FF4E00] font-bold font-mono">MINS</span>
              </div>
              <span className="text-[9px] text-white/30 font-mono mt-1 block">Distractions Prevented</span>
            </div>
            
            <div className="bg-black/60 border border-white/5 p-4 rounded-none min-w-[140px] flex flex-col justify-between">
              <span className="text-[9px] text-white/40 uppercase font-mono tracking-widest">Active Database</span>
              <div className="mt-2 flex items-center gap-1.5 text-white font-mono">
                <Database className="w-4 h-4 text-cyan-400" />
                <span className="text-xs font-bold font-mono">chronos.db</span>
              </div>
              <span className="text-[9px] text-emerald-400/80 font-mono mt-1 block flex items-center gap-1">
                <span className="w-1.5 h-1.5 rounded-full bg-emerald-500 animate-pulse inline-block" /> Isolated Local
              </span>
            </div>

            <div className="bg-black/60 border border-white/5 p-4 rounded-none min-w-[140px] col-span-2 sm:col-span-1 flex flex-col justify-between">
              <span className="text-[9px] text-white/40 uppercase font-mono tracking-widest">Telemetry Rate</span>
              <div className="mt-2 flex items-baseline gap-1">
                <span className="text-2xl font-black text-white font-mono">{stats.ALL}</span>
                <span className="text-[10px] text-white/40 font-mono">EVENTS</span>
              </div>
              <span className="text-[9px] text-white/30 font-mono mt-1 block">Stored Chronologically</span>
            </div>
          </div>
        </div>

        {/* Cyberpunk Status Bar */}
        <div className="mt-6 pt-4 border-t border-white/5 flex flex-wrap gap-4 text-[10px] text-white/40 font-mono uppercase">
          <div className="flex items-center gap-1.5">
            <span className="text-cyan-400 font-bold">FS WATCH:</span>
            <span className="text-white">ACTIVE (Downloads & Configs)</span>
          </div>
          <span className="text-white/10">|</span>
          <div className="flex items-center gap-1.5">
            <span className="text-indigo-400 font-bold">BROWSER INTERCEPT:</span>
            <span className="text-white">WAITING FOR FOCUS</span>
          </div>
          <span className="text-white/10">|</span>
          <div className="flex items-center gap-1.5">
            <span className="text-amber-400 font-bold">SYSTEM TIME:</span>
            <span className="text-white">{new Date(systemTime).toLocaleString()}</span>
          </div>
        </div>
      </div>

      <div className="flex flex-col bg-[#080808] border border-white/10">
        
        {/* Header Actions & Filter Controls */}
        <div className="p-5 border-b border-white/10 space-y-4">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <Terminal className="w-5 h-5 text-[#FF4E00]" />
              <span className="text-xs font-bold font-mono tracking-widest text-white uppercase">
                TELEMETRY_FLIGHT_DECK.log
              </span>
            </div>
            <div className="flex items-center gap-2">
              <button
                onClick={() => setActiveTab('timeline')}
                className={`text-[10px] font-bold uppercase tracking-wider px-3 py-1 font-mono transition border ${
                  activeTab === 'timeline'
                    ? 'border-[#FF4E00] text-[#FF4E00] bg-[#FF4E00]/5'
                    : 'border-white/5 text-white/40 hover:text-white'
                }`}
              >
                Timeline Feed
              </button>
              <button
                onClick={() => setActiveTab('insights')}
                className={`text-[10px] font-bold uppercase tracking-wider px-3 py-1 font-mono transition border ${
                  activeTab === 'insights'
                    ? 'border-[#FF4E00] text-[#FF4E00] bg-[#FF4E00]/5'
                    : 'border-white/5 text-white/40 hover:text-white'
                }`}
              >
                Frequency Spark
              </button>
              <button
                onClick={async () => {
                  try {
                    const res = await fetch(`${API_BASE}/api/context/export`);
                    const data = await res.json();
                    const blob = new Blob([JSON.stringify(data, null, 2)], { type: 'application/json' });
                    const url = URL.createObjectURL(blob);
                    const a = document.createElement('a');
                    a.href = url;
                    a.download = `chronos-export-${Date.now()}.chronos.json`;
                    document.body.appendChild(a);
                    a.click();
                    document.body.removeChild(a);
                    URL.revokeObjectURL(url);
                  } catch (e) {
                    console.error('Export failed:', e);
                  }
                }}
                className="text-[10px] font-bold uppercase tracking-wider px-3 py-1 font-mono transition border border-emerald-500/30 text-emerald-400 hover:bg-emerald-500/10 flex items-center gap-1"
              >
                <Download className="w-3 h-3" /> Export .chronos
              </button>
            </div>
          </div>

          {/* Filter Pills */}
          <div className="flex flex-wrap items-center gap-2">
            <button
              onClick={() => setFilterCategory('ALL')}
              className={`text-[10px] font-mono font-bold uppercase tracking-wide px-3 py-1.5 transition rounded-none flex items-center gap-1.5 ${
                filterCategory === 'ALL'
                  ? 'bg-[#FF4E00] text-black'
                  : 'bg-neutral-900 border border-white/10 text-white/60 hover:text-white hover:border-white/20'
              }`}
            >
              All Events
              <span className={`text-[9px] px-1 rounded ${filterCategory === 'ALL' ? 'bg-black/25 text-black' : 'bg-neutral-800 text-white/40'}`}>
                {stats.ALL}
              </span>
            </button>

            <button
              onClick={() => setFilterCategory('FS_MONITOR')}
              className={`text-[10px] font-mono font-bold uppercase tracking-wide px-3 py-1.5 transition rounded-none flex items-center gap-1.5 ${
                filterCategory === 'FS_MONITOR'
                  ? 'bg-cyan-500 text-black'
                  : 'bg-neutral-900 border border-white/10 text-cyan-400 hover:text-cyan-300 hover:border-cyan-500/30'
              }`}
            >
              <FileCode className="w-3.5 h-3.5" />
              Filesystem
              <span className={`text-[9px] px-1 rounded ${filterCategory === 'FS_MONITOR' ? 'bg-black/25 text-black' : 'bg-neutral-800 text-cyan-400/60'}`}>
                {stats.FS_MONITOR}
              </span>
            </button>

            <button
              onClick={() => setFilterCategory('BROWSER_MONITOR')}
              className={`text-[10px] font-mono font-bold uppercase tracking-wide px-3 py-1.5 transition rounded-none flex items-center gap-1.5 ${
                filterCategory === 'BROWSER_MONITOR'
                  ? 'bg-indigo-500 text-black'
                  : 'bg-neutral-900 border border-white/10 text-indigo-400 hover:text-indigo-300 hover:border-indigo-500/30'
              }`}
            >
              <Globe className="w-3.5 h-3.5" />
              Browser
              <span className={`text-[9px] px-1 rounded ${filterCategory === 'BROWSER_MONITOR' ? 'bg-black/25 text-black' : 'bg-neutral-800 text-indigo-400/60'}`}>
                {stats.BROWSER_MONITOR}
              </span>
            </button>

            <button
              onClick={() => setFilterCategory('TERMINAL_MONITOR')}
              className={`text-[10px] font-mono font-bold uppercase tracking-wide px-3 py-1.5 transition rounded-none flex items-center gap-1.5 ${
                filterCategory === 'TERMINAL_MONITOR'
                  ? 'bg-amber-500 text-black'
                  : 'bg-neutral-900 border border-white/10 text-amber-400 hover:text-amber-300 hover:border-amber-500/30'
              }`}
            >
              <Terminal className="w-3.5 h-3.5" />
              Terminal
              <span className={`text-[9px] px-1 rounded ${filterCategory === 'TERMINAL_MONITOR' ? 'bg-black/25 text-black' : 'bg-neutral-800 text-amber-400/60'}`}>
                {stats.TERMINAL_MONITOR || 0}
              </span>
            </button>

            <button
              onClick={() => setFilterCategory('IDE_MONITOR')}
              className={`text-[10px] font-mono font-bold uppercase tracking-wide px-3 py-1.5 transition rounded-none flex items-center gap-1.5 ${
                filterCategory === 'IDE_MONITOR'
                  ? 'bg-cyan-500 text-black'
                  : 'bg-neutral-900 border border-white/10 text-cyan-400 hover:text-cyan-300 hover:border-cyan-500/30'
              }`}
            >
              <FileCode className="w-3.5 h-3.5" />
              IDE / Editor
              <span className={`text-[9px] px-1 rounded ${filterCategory === 'IDE_MONITOR' ? 'bg-black/25 text-black' : 'bg-neutral-800 text-cyan-400/60'}`}>
                {stats.IDE_MONITOR || 0}
              </span>
            </button>

            <button
              onClick={() => setFilterCategory('COMMUNICATION_MONITOR')}
              className={`text-[10px] font-mono font-bold uppercase tracking-wide px-3 py-1.5 transition rounded-none flex items-center gap-1.5 ${
                filterCategory === 'COMMUNICATION_MONITOR'
                  ? 'bg-rose-500 text-black'
                  : 'bg-neutral-900 border border-white/10 text-rose-400 hover:text-rose-300 hover:border-rose-500/30'
              }`}
            >
              <MessageSquare className="w-3.5 h-3.5" />
              Chat / Comm
              <span className={`text-[9px] px-1 rounded ${filterCategory === 'COMMUNICATION_MONITOR' ? 'bg-black/25 text-black' : 'bg-neutral-800 text-rose-400/60'}`}>
                {stats.COMMUNICATION_MONITOR || 0}
              </span>
            </button>

            <button
              onClick={() => setFilterCategory('APP_MONITOR')}
              className={`text-[10px] font-mono font-bold uppercase tracking-wide px-3 py-1.5 transition rounded-none flex items-center gap-1.5 ${
                filterCategory === 'APP_MONITOR'
                  ? 'bg-[#888888] text-black'
                  : 'bg-neutral-900 border border-white/10 text-white/60 hover:text-white hover:border-white/20'
              }`}
            >
              <Cpu className="w-3.5 h-3.5" />
              Other Apps
              <span className={`text-[9px] px-1 rounded ${filterCategory === 'APP_MONITOR' ? 'bg-black/25 text-black' : 'bg-neutral-800 text-white/40'}`}>
                {stats.APP_MONITOR || 0}
              </span>
            </button>

            <button
              onClick={() => setFilterCategory('COGNITIVE_TRACE')}
              className={`text-[10px] font-mono font-bold uppercase tracking-wide px-3 py-1.5 transition rounded-none flex items-center gap-1.5 ${
                filterCategory === 'COGNITIVE_TRACE'
                  ? 'bg-amber-500 text-black'
                  : 'bg-neutral-900 border border-white/10 text-amber-400 hover:text-amber-300 hover:border-amber-500/30'
              }`}
            >
              <CheckSquare className="w-3.5 h-3.5" />
              Checkpoints
              <span className={`text-[9px] px-1 rounded ${filterCategory === 'COGNITIVE_TRACE' ? 'bg-black/25 text-black' : 'bg-neutral-800 text-amber-400/60'}`}>
                {stats.COGNITIVE_TRACE}
              </span>
            </button>

            <button
              onClick={() => setFilterCategory('RECOVERY_PLAN')}
              className={`text-[10px] font-mono font-bold uppercase tracking-wide px-3 py-1.5 transition rounded-none flex items-center gap-1.5 ${
                filterCategory === 'RECOVERY_PLAN'
                  ? 'bg-orange-500 text-black'
                  : 'bg-neutral-900 border border-white/10 text-orange-400 hover:text-orange-300 hover:border-orange-500/30'
              }`}
            >
              <Sliders className="w-3.5 h-3.5" />
              Recovery Plans
              <span className={`text-[9px] px-1 rounded ${filterCategory === 'RECOVERY_PLAN' ? 'bg-black/25 text-black' : 'bg-neutral-800 text-orange-400/60'}`}>
                {stats.RECOVERY_PLAN}
              </span>
            </button>
          </div>

          {/* Manual Ingest Widget */}
          <div className="border border-white/5 bg-neutral-950/40 p-3 mb-2 font-mono text-xs">
            {!showManualIngest ? (
              <button
                type="button"
                onClick={() => setShowManualIngest(true)}
                className="w-full flex items-center justify-center gap-2 py-1.5 border border-dashed border-white/10 hover:border-[#FF4E00]/40 text-white/60 hover:text-[#FF4E00] transition duration-150"
              >
                <Plus className="w-3.5 h-3.5 text-[#FF4E00]" />
                <span>COPY TO CHRONOS (MANUAL INGEST)</span>
              </button>
            ) : (
              <form onSubmit={handleManualIngest} className="space-y-3">
                <div className="flex items-center justify-between border-b border-white/5 pb-2">
                  <span className="text-[10px] font-bold text-[#FF4E00] uppercase tracking-wider">📋 Manual Ingestion Portal</span>
                  <button
                    type="button"
                    onClick={() => {
                      setShowManualIngest(false);
                      setIngestError(null);
                    }}
                    className="text-white/40 hover:text-white"
                  >
                    <X className="w-3.5 h-3.5" />
                  </button>
                </div>

                <div className="space-y-2">
                  <label className="block text-[10px] text-white/50 uppercase">Type of telemetry node</label>
                  <div className="grid grid-cols-3 gap-1">
                    {(['URL', 'FILE', 'TEXT'] as const).map((type) => (
                      <button
                        key={type}
                        type="button"
                        onClick={() => {
                          setManualType(type);
                          setManualDetail('');
                        }}
                        className={`py-1 text-center font-bold tracking-wide transition border text-[10px] ${
                          manualType === type
                            ? 'border-[#FF4E00] text-[#FF4E00] bg-[#FF4E00]/5'
                            : 'border-white/5 text-white/40 hover:text-white'
                        }`}
                      >
                        {type}
                      </button>
                    ))}
                  </div>
                </div>

                <div className="space-y-1">
                  <label className="block text-[10px] text-white/50 uppercase">Display Title</label>
                  <input
                    type="text"
                    required
                    value={manualTitle}
                    onChange={(e) => setManualTitle(e.target.value)}
                    placeholder={
                      manualType === 'URL'
                        ? 'e.g. GitHub - Chronos Issue #12'
                        : manualType === 'FILE'
                        ? 'e.g. backend/server.rs'
                        : 'e.g. Notes on DB design'
                    }
                    className="w-full bg-[#111] border border-white/10 px-2 py-1 text-xs text-white placeholder-white/20 focus:outline-none focus:border-[#FF4E00]"
                  />
                </div>

                <div className="space-y-1">
                  <label className="block text-[10px] text-white/50 uppercase">
                    {manualType === 'URL' ? 'URL Link' : manualType === 'FILE' ? 'File Absolute Path' : 'Context Details / Text'}
                  </label>
                  {manualType === 'TEXT' ? (
                    <textarea
                      required
                      rows={3}
                      value={manualDetail}
                      onChange={(e) => setManualDetail(e.target.value)}
                      placeholder="Paste text snippet or cognitive notes here..."
                      className="w-full bg-[#111] border border-white/10 px-2 py-1 text-xs text-white placeholder-white/20 focus:outline-none focus:border-[#FF4E00] resize-none"
                    />
                  ) : (
                    <input
                      type="text"
                      required
                      value={manualDetail}
                      onChange={(e) => setManualDetail(e.target.value)}
                      placeholder={
                        manualType === 'URL'
                          ? 'e.g. https://github.com/Arshdeep2k5/Chronos/issues/12'
                          : 'e.g. D:/Chronos_Hackathon/src-tauri/src/server.rs'
                      }
                      className="w-full bg-[#111] border border-white/10 px-2 py-1 text-xs text-white placeholder-white/20 focus:outline-none focus:border-[#FF4E00]"
                    />
                  )}
                </div>

                {ingestError && (
                  <div className="text-[10px] text-red-400 bg-red-950/20 border border-red-500/20 px-2 py-1">
                    {ingestError}
                  </div>
                )}

                <div className="flex items-center justify-end gap-2 pt-1">
                  <button
                    type="button"
                    onClick={() => {
                      setShowManualIngest(false);
                      setIngestError(null);
                    }}
                    className="px-3 py-1 border border-white/5 text-white/40 hover:text-white hover:border-white/10 transition"
                  >
                    Cancel
                  </button>
                  <button
                    type="submit"
                    disabled={ingesting}
                    className="px-3 py-1 bg-[#FF4E00] hover:bg-[#FF4E00]/80 text-black font-bold transition disabled:opacity-50"
                  >
                    {ingesting ? 'Ingesting...' : 'Ingest Context'}
                  </button>
                </div>
              </form>
            )}
          </div>

          {/* Search Input */}
          <div className="relative">
            <span className="absolute inset-y-0 left-0 pl-3.5 flex items-center pointer-events-none text-white/30">
              <Search className="w-4 h-4" />
            </span>
            <input
              type="text"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder="Search raw logs by query, filenames, content terms..."
              className="w-full bg-[#111] border border-white/10 pl-10 pr-4 py-2 font-mono text-xs text-white placeholder-white/20 focus:outline-none focus:border-[#FF4E00] focus:ring-1 focus:ring-[#FF4E00]"
            />
            {searchQuery && (
              <button
                onClick={() => setSearchQuery('')}
                className="absolute inset-y-0 right-0 pr-3.5 flex items-center text-white/30 hover:text-white"
              >
                <X className="w-4 h-4" />
              </button>
            )}
          </div>
        </div>

        {/* Timeline Contents */}
        <div className="p-5 flex-1 max-h-[550px] overflow-y-auto min-h-[300px]">
          <AnimatePresence mode="popLayout">
            {activeTab === 'timeline' ? (
              <div className="space-y-3">
                {filteredLogs.length === 0 ? (
                  <div className="py-16 text-center space-y-2 border border-dashed border-white/5 bg-neutral-950/20">
                    <Terminal className="w-8 h-8 text-white/15 mx-auto" />
                    <p className="text-xs font-mono text-white/40">No matching telemetry events found.</p>
                    <p className="text-[10px] font-mono text-white/25">Try modifying filters or working in your workspace.</p>
                  </div>
                ) : (
                  filteredLogs.map((log, idx) => {
                    const isGrouped = log.is_grouped && log.group_count && log.group_count > 1;

                    let focusMode = log.focus_mode || 'Unassigned';
                    let projectName = log.project_name || 'System';
                    
                    let bgClass = "bg-[#1F2937]/40";
                    let borderClass = "border-gray-500/30";
                    let accentClass = "border-l-4 border-gray-500";
                    let titleColor = "text-gray-300";
                    let headerIcon = <Terminal className="w-4 h-4 text-gray-400" />;
                    let headerText = `${projectName} - ${focusMode}`;
                    
                    if (focusMode === 'DeepWork') {
                      bgClass = "bg-[#064e3b]/20 hover:bg-[#064e3b]/30"; // Emerald 900
                      borderClass = "border-emerald-500/20";
                      accentClass = "border-l-4 border-emerald-500";
                      titleColor = "text-emerald-300";
                      headerIcon = <FileCode className="w-4 h-4 text-emerald-400" />;
                      headerText = `🛠 ${projectName} - Deep Work`;
                    } else if (focusMode === 'ProjectResearch') {
                      bgClass = "bg-[#1e3a8a]/20 hover:bg-[#1e3a8a]/30"; // Blue 900
                      borderClass = "border-blue-500/20";
                      accentClass = "border-l-4 border-blue-500";
                      titleColor = "text-blue-300";
                      headerIcon = <Search className="w-4 h-4 text-blue-400" />;
                      headerText = `🔬 ${projectName} - Research`;
                    } else if (focusMode === 'Distraction') {
                      bgClass = "bg-[#111827]/60"; // Gray 900
                      borderClass = "border-gray-700/50";
                      accentClass = "border-l-4 border-gray-600";
                      titleColor = "text-gray-500";
                      headerIcon = <Globe className="w-4 h-4 text-gray-600" />;
                      headerText = `🚫 Non-Focused / Distraction`;
                    } else if (focusMode === 'Unassigned') {
                      bgClass = "bg-[#1e293b]/30 hover:bg-[#1e293b]/40"; // Slate 800
                      borderClass = "border-slate-500/20";
                      accentClass = "border-l-4 border-slate-500";
                      titleColor = "text-slate-300";
                      headerIcon = <Globe className="w-4 h-4 text-slate-400" />;
                      headerText = `🌐 General Browsing`;
                    } else {
                      // System events
                      bgClass = "bg-[#2e1065]/20 hover:bg-[#2e1065]/30"; // Violet 900
                      borderClass = "border-violet-500/20";
                      accentClass = "border-l-4 border-violet-500";
                      titleColor = "text-violet-300";
                      headerIcon = <Cpu className="w-4 h-4 text-violet-400" />;
                      headerText = `⚙️ System Context`;
                    }
                    
                    const groupKey = getLogKey(log);
                    const isExpanded = expandedGroups[groupKey] !== false; // Default expanded unless distraction?
                    // Actually, let's track state natively:
                    const isActuallyExpanded = expandedGroups[groupKey] || (focusMode === 'DeepWork' && expandedGroups[groupKey] === undefined);

                    return (
                      <motion.div
                        key={groupKey}
                        layout
                        initial={{ opacity: 0, y: 12 }}
                        animate={{ opacity: focusMode === 'Distraction' ? 0.6 : 1, y: 0 }}
                        exit={{ opacity: 0 }}
                        transition={{ duration: 0.2 }}
                        className={`p-3 relative border shadow-sm transition-all overflow-hidden ${bgClass} ${borderClass} ${accentClass}`}
                      >
                        <div className="flex items-start justify-between gap-4 mb-2">
                          <div className="flex items-center gap-2">
                            {headerIcon}
                            <h3 className={`text-xs font-bold font-mono tracking-wide ${titleColor}`}>
                              {headerText}
                            </h3>
                            {isGrouped && (
                              <span className="text-[10px] text-white/30 font-mono ml-2">
                                ({log.group_count} items)
                              </span>
                            )}
                          </div>
                          <div className="flex flex-col items-end gap-2 shrink-0">
                            <div className="flex items-center gap-3">
                              <span className="text-[10px] font-mono text-white/30">
                                {log.timestamp}
                              </span>
                              <button
                                onClick={() => handleRemoveLog(log.id || null, log.event_type, log.ids)}
                                className="text-white/10 hover:text-red-400 transition-colors"
                                title="Delete from timeline"
                              >
                                <Trash2 className="w-3.5 h-3.5" />
                              </button>
                            </div>
                            <button
                               onClick={() => setSelectedLog(log)}
                               className="px-1.5 py-0.5 rounded bg-red-500/10 border border-red-500/30 hover:bg-red-500/20 text-[9px] uppercase tracking-wider text-red-400 transition-colors"
                               title="Inspect JSON Payload"
                            >
                              Inspect
                            </button>
                          </div>
                        </div>

                        {/* Top-level summary or single item */}
                        {!isGrouped && (
                           <div className="pl-6 pr-28 text-[11px] font-mono text-white/60 mb-2 truncate">
                              {log.display_name} {log.detail ? `— ${log.detail}` : ""}
                           </div>
                        )}

                        {isGrouped && (
                          <div className="pl-6 mb-1">
                            <button
                              onClick={() => toggleGroup(groupKey)}
                              className={`text-[9px] font-bold uppercase py-0.5 px-2 border transition-all hover:bg-white/10 ${titleColor} ${borderClass}`}
                            >
                              {isActuallyExpanded ? "Hide Details" : "Show Details"}
                            </button>
                          </div>
                        )}

                        {/* Sub-items */}
                        {isGrouped && isActuallyExpanded && log.sub_items && (
                          <div className={`mt-2 pl-6 space-y-1 py-1`}>
                            {log.sub_items.map((sub, sIdx) => {
                               let subIcon = <Globe className="w-3 h-3 text-white/30 shrink-0" />;
                               if (sub.category === 'IDE_MONITOR' || sub.category === 'FS_MONITOR') subIcon = <FileCode className="w-3 h-3 text-white/30 shrink-0" />;
                               if (sub.category === 'TERMINAL_MONITOR') subIcon = <Terminal className="w-3 h-3 text-white/30 shrink-0" />;
                               
                               return (
                                <div key={sub.id || sIdx} className="text-[10px] font-mono flex items-start gap-2 text-white/40 hover:text-white/60 transition-colors">
                                  {subIcon}
                                  <span className="truncate flex-1 pr-8" title={sub.url}>
                                    {sub.title || sub.url}
                                  </span>
                                  <span className="shrink-0 text-[8.5px] text-white/20">
                                    {sub.timestamp ? sub.timestamp.split(',').pop().trim() : ""}
                                  </span>
                                </div>
                               );
                            })}
                          </div>
                        )}
                      </motion.div>
                    );
                  })
                )}
              </div>
            ) : (
              /* Visual Pulse Frequency Spark Chart */
              <div className="space-y-6 py-6 font-mono">
                <div>
                  <h3 className="text-xs font-bold uppercase text-white tracking-widest flex items-center gap-1.5">
                    <Activity className="w-4 h-4 text-[#FF4E00]" /> Telemetry Impulse Frequency Sparkline
                  </h3>
                  <p className="text-[10px] text-white/40 mt-1">
                    Visualizing log injection weights over the most recent 12 background operations.
                  </p>
                </div>

                {activityPulses.length === 0 ? (
                  <div className="py-12 text-center text-xs text-white/30">
                    Telemetry empty. Cannot render waveform.
                  </div>
                ) : (
                  <div className="bg-[#0C0C0C] border border-white/5 p-6 space-y-4">
                    {/* Interactive Bar graph */}
                    <div className="h-32 flex items-end gap-3.5 border-b border-white/10 pb-2.5 px-4">
                      {activityPulses.map((pulse) => {
                        let barColor = 'bg-emerald-500/50 shadow-[0_0_8px_rgba(16,185,129,0.3)]';
                        if (pulse.category === 'FS_MONITOR') barColor = 'bg-cyan-500/50 shadow-[0_0_8px_rgba(6,182,212,0.3)]';
                        if (pulse.category === 'BROWSER_MONITOR') barColor = 'bg-indigo-500/50 shadow-[0_0_8px_rgba(99,102,241,0.3)]';
                        if (pulse.category === 'TERMINAL_MONITOR') barColor = 'bg-amber-500/50 shadow-[0_0_8px_rgba(245,158,11,0.3)]';
                        if (pulse.category === 'IDE_MONITOR') barColor = 'bg-cyan-500/50 shadow-[0_0_8px_rgba(6,182,212,0.3)]';
                        if (pulse.category === 'COMMUNICATION_MONITOR') barColor = 'bg-rose-500/50 shadow-[0_0_8px_rgba(244,63,94,0.3)]';
                        if (pulse.category === 'APP_MONITOR') barColor = 'bg-neutral-500/50 shadow-[0_0_8px_rgba(115,115,115,0.3)]';
                        if (pulse.category === 'COGNITIVE_TRACE') barColor = 'bg-amber-500/50 shadow-[0_0_8px_rgba(245,158,11,0.3)]';
                        if (pulse.category === 'RECOVERY_PLAN') barColor = 'bg-orange-600/70 shadow-[0_0_10px_rgba(234,88,12,0.4)]';

                        return (
                          <div key={pulse.id} className="flex-1 h-full flex flex-col justify-end items-center gap-2 group relative">
                            {/* Tooltip */}
                            <div className="absolute bottom-full mb-2 bg-neutral-950 text-white text-[9px] px-2 py-1 border border-white/10 rounded-none whitespace-nowrap opacity-0 group-hover:opacity-100 transition pointer-events-none z-10">
                              <span className="font-bold">{pulse.category}</span>
                              <span className="block text-[8px] text-white/40">{pulse.time}</span>
                            </div>
                            <div
                              style={{ height: `${pulse.height}%` }}
                              className={`w-full ${barColor} transition-all duration-500 hover:brightness-125`}
                            />
                          </div>
                        );
                      })}
                    </div>

                    <div className="flex justify-between text-[8px] text-white/30 uppercase">
                      <span>&lt;&lt; Past Operations (t-12)</span>
                      <span>Latest Telemetry Pulse &gt;&gt;</span>
                    </div>
                  </div>
                )}

                {/* Volumetric Breakdown */}
                <div className="grid grid-cols-2 sm:grid-cols-4 gap-4">
                  <div className="bg-black/40 border border-white/5 p-4 flex flex-col justify-between">
                    <span className="text-[9px] text-white/30 uppercase font-mono">Filesystem Ratio</span>
                    <span className="text-xl font-black font-mono text-cyan-400 mt-1">
                      {stats.ALL > 0 ? ((stats.FS_MONITOR / stats.ALL) * 100).toFixed(0) : 0}%
                    </span>
                  </div>
                  <div className="bg-black/40 border border-white/5 p-4 flex flex-col justify-between">
                    <span className="text-[9px] text-white/30 uppercase font-mono">Browser Ratio</span>
                    <span className="text-xl font-black font-mono text-indigo-400 mt-1">
                      {stats.ALL > 0 ? ((stats.BROWSER_MONITOR / stats.ALL) * 100).toFixed(0) : 0}%
                    </span>
                  </div>
                  <div className="bg-black/40 border border-white/5 p-4 flex flex-col justify-between">
                    <span className="text-[9px] text-white/30 uppercase font-mono">Terminal Ratio</span>
                    <span className="text-xl font-black font-mono text-amber-400 mt-1">
                      {stats.ALL > 0 ? (((stats.TERMINAL_MONITOR || 0) / stats.ALL) * 100).toFixed(0) : 0}%
                    </span>
                  </div>
                  <div className="bg-black/40 border border-white/5 p-4 flex flex-col justify-between">
                    <span className="text-[9px] text-white/30 uppercase font-mono">IDE Ratio</span>
                    <span className="text-xl font-black font-mono text-cyan-400 mt-1">
                      {stats.ALL > 0 ? (((stats.IDE_MONITOR || 0) / stats.ALL) * 100).toFixed(0) : 0}%
                    </span>
                  </div>
                  <div className="bg-black/40 border border-white/5 p-4 flex flex-col justify-between">
                    <span className="text-[9px] text-white/30 uppercase font-mono">Chat / Comm Ratio</span>
                    <span className="text-xl font-black font-mono text-rose-400 mt-1">
                      {stats.ALL > 0 ? (((stats.COMMUNICATION_MONITOR || 0) / stats.ALL) * 100).toFixed(0) : 0}%
                    </span>
                  </div>
                  <div className="bg-black/40 border border-white/5 p-4 flex flex-col justify-between">
                    <span className="text-[9px] text-white/30 uppercase font-mono">Other Apps Ratio</span>
                    <span className="text-xl font-black font-mono text-neutral-400 mt-1">
                      {stats.ALL > 0 ? (((stats.APP_MONITOR || 0) / stats.ALL) * 100).toFixed(0) : 0}%
                    </span>
                  </div>
                  <div className="bg-black/40 border border-white/5 p-4 flex flex-col justify-between">
                    <span className="text-[9px] text-white/30 uppercase font-mono">Checkpoint Ratio</span>
                    <span className="text-xl font-black font-mono text-amber-400 mt-1">
                      {stats.ALL > 0 ? ((stats.COGNITIVE_TRACE / stats.ALL) * 100).toFixed(0) : 0}%
                    </span>
                  </div>
                  <div className="bg-black/40 border border-white/5 p-4 flex flex-col justify-between">
                    <span className="text-[9px] text-white/30 uppercase font-mono">Recovery Plans</span>
                    <span className="text-xl font-black font-mono text-orange-500 mt-1">
                      {stats.ALL > 0 ? ((stats.RECOVERY_PLAN / stats.ALL) * 100).toFixed(0) : 0}%
                    </span>
                  </div>
                </div>
              </div>
            )}
          </AnimatePresence>
        </div>
      </div>

      {/* 4. DETAILS INSPECT MODAL / SLIDE OVER */}
      <AnimatePresence>
        {selectedLog && (
          <div className="fixed inset-0 bg-black/70 backdrop-blur-sm flex items-center justify-end z-50">
            <motion.div
              initial={{ x: '100%' }}
              animate={{ x: 0 }}
              exit={{ x: '100%' }}
              transition={{ type: 'spring', damping: 25, stiffness: 200 }}
              className="w-full max-w-xl h-full bg-[#0A0A0A] border-l border-white/10 p-6 flex flex-col justify-between shadow-2xl relative"
            >
              <div className="space-y-6 overflow-y-auto flex-1 pr-2">
                <div className="flex items-center justify-between border-b border-white/10 pb-4">
                  <div className="flex items-center gap-2">
                    <Terminal className="w-5 h-5 text-[#FF4E00]" />
                    <h3 className="text-xs font-bold uppercase tracking-widest font-mono text-white">
                      Inspect Telemetry Event
                    </h3>
                  </div>
                  <button
                    onClick={() => setSelectedLog(null)}
                    className="p-1.5 hover:bg-white/5 border border-transparent hover:border-white/10 text-white/60 hover:text-white transition rounded"
                  >
                    <X className="w-4 h-4" />
                  </button>
                </div>

                {/* Event Summary */}
                <div className="bg-neutral-900/40 border border-white/5 p-4 space-y-4">
                  <div className="grid grid-cols-2 gap-4 text-xs font-mono">
                    <div>
                      <span className="text-white/30 block text-[9px] uppercase tracking-widest">Category</span>
                      <span className="text-[#FF4E00] font-bold">{selectedLog.category}</span>
                    </div>
                    <div>
                      <span className="text-white/30 block text-[9px] uppercase tracking-widest">Event Type</span>
                      <span className="text-white">{selectedLog.event_type}</span>
                    </div>
                    <div>
                      <span className="text-white/30 block text-[9px] uppercase tracking-widest">Intercepted Time</span>
                      <span className="text-white">{formatRelativeTimestamp(selectedLog.raw_timestamp, selectedLog.timestamp, systemTime)}</span>
                    </div>
                    <div>
                      <span className="text-white/30 block text-[9px] uppercase tracking-widest">Epoch Timestamp</span>
                      <span className="text-white/60 text-[10px] break-all">{selectedLog.raw_timestamp || 'N/A'}</span>
                    </div>
                  </div>

                  <div className="border-t border-white/5 pt-4 space-y-1.5 font-mono text-xs">
                    <span className="text-white/30 text-[9px] uppercase tracking-widest block">Event Title / Key</span>
                    <span className="text-white font-bold block">{selectedLog.display_name}</span>
                  </div>

                  <div className="space-y-1.5 font-mono text-xs">
                    <span className="text-white/30 text-[9px] uppercase tracking-widest block">Details</span>
                    <p className="text-white/70 leading-relaxed bg-black/40 border border-white/5 p-3 rounded">
                      {selectedLog.detail}
                    </p>
                  </div>
                </div>

                {/* Raw database record mock/view */}
                <div className="space-y-2">
                  <div className="flex items-center gap-1.5">
                    <Info className="w-4 h-4 text-cyan-400" />
                    <span className="text-[10px] font-bold text-white/50 uppercase font-mono tracking-wider">
                      SQLite Raw Payload Schema
                    </span>
                  </div>
                  <pre className="p-4 bg-black border border-white/10 text-[10px] font-mono text-cyan-300/90 overflow-x-auto select-all max-h-[300px]">
                    {JSON.stringify({
                      source: "SQLite Database Local",
                      table: selectedLog.category === 'FS_MONITOR' ? 'context_events' :
                             selectedLog.category === 'BROWSER_MONITOR' ? 'browser_sessions' :
                             selectedLog.category === 'COGNITIVE_TRACE' ? 'project_checkpoints' : 'recovery_plans',
                      schema: {
                        timestamp: selectedLog.raw_timestamp,
                        display_name: selectedLog.display_name,
                        category: selectedLog.category,
                        event_type: selectedLog.event_type,
                        details: selectedLog.detail
                      },
                      isolation_level: "SERIALIZABLE",
                      journal_mode: "WAL"
                    }, null, 2)}
                  </pre>
                </div>
              </div>

              <div className="border-t border-white/10 pt-4 mt-6">
                <button
                  onClick={() => setSelectedLog(null)}
                  className="w-full py-2.5 bg-[#FF4E00] hover:bg-white text-black font-bold uppercase font-mono text-xs tracking-wider transition cursor-pointer text-center"
                >
                  Close Inspector
                </button>
              </div>
            </motion.div>
          </div>
        )}
      </AnimatePresence>

    </div>
  );
}
