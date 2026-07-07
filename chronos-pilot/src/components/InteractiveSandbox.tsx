/**
 * @license
 * SPDX-License-Identifier: Apache-2.0
 */

import React, { useState } from 'react';
import { Play, Download, Search, AlertTriangle, RefreshCw, CheckSquare, Clock } from 'lucide-react';

interface InteractiveSandboxProps {
  systemTime: string;
  hasApiKey: boolean;
  onRefresh: () => void;
  onTimeTravel: (hours: number) => void;
}

export default function InteractiveSandbox({
  systemTime,
  hasApiKey,
  onRefresh,
  onTimeTravel
}: InteractiveSandboxProps) {
  const [selectedProjectId, setSelectedProjectId] = useState<number>(1);
  const [accomplished, setAccomplished] = useState('');
  const [blocked, setBlocked] = useState('');
  const [nextSteps, setNextSteps] = useState('');
  const [searchQueryInput, setSearchQueryInput] = useState('');
  const [simulatedPdfName, setSimulatedPdfName] = useState('ML_Theoretical_Foundations.pdf');
  const [statusMessage, setStatusMessage] = useState<{ text: string; type: 'success' | 'info' | 'warn' | null }>({
    text: '',
    type: null
  });
  const [isSubmitting, setIsSubmitting] = useState(false);

  const triggerToast = (text: string, type: 'success' | 'info' | 'warn') => {
    setStatusMessage({ text, type });
    setTimeout(() => {
      setStatusMessage({ text: '', type: null });
    }, 5000);
  };

  const handleDownloadSimulation = async () => {
    try {
      const res = await fetch('/api/perception/ingest', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          source: 'manual',
          payload: {
            entity_key: `FILE:notebooks/${simulatedPdfName}`,
            display_name: simulatedPdfName
          }
        })
      });
      const data = await res.json();
      if (data.ok) {
        triggerToast(`Document '${simulatedPdfName}' ingested successfully. Tick loop triggered!`, 'success');
        onRefresh();
      }
    } catch (e) {
      console.error(e);
    }
  };

  const handleSearchSimulation = async () => {
    if (!searchQueryInput.trim()) return;
    try {
      const res = await fetch('/api/perception/ingest', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          source: 'browser',
          payload: {
            type: 'url_changed',
            url: `https://www.google.com/search?q=${encodeURIComponent(searchQueryInput)}`,
            title: `Google Search: ${searchQueryInput}`,
            tab_id: 1
          }
        })
      });
      const data = await res.json();
      if (data.ok) {
        triggerToast(`Telemetry intercepted: "${searchQueryInput}" ingested to search graph`, 'info');
        setSearchQueryInput('');
        onRefresh();
      }
    } catch (e) {
      console.error(e);
    }
  };

  const handleCheckpointSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!accomplished.trim()) return;
    setIsSubmitting(true);
    try {
      const res = await fetch('/api/perception/ingest', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          source: 'manual',
          payload: {
            display_name: `Checkpoint: ${accomplished} | Blocked: ${blocked || 'None'} | Next: ${nextSteps || 'None'}`
          }
        })
      });
      const data = await res.json();
      if (data.ok) {
        triggerToast('Human ground-truth checkpoint submitted via perception pipeline!', 'success');
        setAccomplished('');
        setBlocked('');
        setNextSteps('');
        onRefresh();
      }
    } catch (e) {
      console.error(e);
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleSystemPurge = async () => {
    if (confirm('Are you sure you want to purge system logs? This will reset all telemetry. (Privacy sovereignty demo)')) {
      triggerToast('Privacy sovereignty: Resetting frontend session state.', 'warn');
      onRefresh();
    }
  };

  const formattedDate = (str: string) => {
    try {
      return new Date(str).toLocaleString('en-US', {
        month: 'short',
        day: 'numeric',
        hour: '2-digit',
        minute: '2-digit',
        second: '2-digit'
      });
    } catch (e) {
      return str;
    }
  };

  return (
    <div id="sandbox-panel" className="bg-[#0F0F0F] border border-white/10 p-6 shadow-2xl rounded-none">
      <div className="flex items-center justify-between border-b border-white/10 pb-4 mb-4">
        <div>
          <h2 className="font-display font-black italic text-lg text-white flex items-center gap-2">
            <Play className="w-4 h-4 text-[#FF4E00] fill-[#FF4E00]" />
            TELEMETRY_SANDBOX
          </h2>
          <p className="text-[10px] uppercase tracking-widest text-white/40 mt-1">Simulate user events & workspace indicators</p>
        </div>
        <div className="flex flex-col items-end font-mono">
          <span className="text-[9px] text-white/30 tracking-widest">DAEMON PORT</span>
          <span className="text-[11px] font-bold text-[#FF4E00]">48120_OK</span>
        </div>
      </div>

      {/* Clock Controls */}
      <div className="bg-[#121212] border border-white/5 p-4 mb-6 rounded-none">
        <div className="flex items-center justify-between border-b border-white/5 pb-2">
          <div className="flex items-center gap-2">
            <Clock className="w-3.5 h-3.5 text-white/40" />
            <span className="text-[10px] uppercase tracking-wider text-white/60">Simulated Daemon Clock</span>
          </div>
          <span className="text-[10px] font-mono font-bold text-[#FF4E00] tracking-widest">
            {formattedDate(systemTime)}
          </span>
        </div>
        <p className="text-[11px] text-white/50 mt-2.5 leading-relaxed">
          Chronos Pilot monitors working velocity gaps. Shift the timeframe forward to decay attention weights and test failure cascades.
        </p>
        <div className="grid grid-cols-2 gap-3 mt-4">
          <button
            id="time-travel-24h"
            onClick={() => onTimeTravel(24)}
            className="flex items-center justify-center bg-transparent hover:bg-white hover:text-black text-white text-[10px] uppercase tracking-[0.15em] font-bold py-2 px-3 border border-white/10 transition cursor-pointer"
          >
            Travel +24h
          </button>
          <button
            id="time-travel-72h"
            onClick={() => onTimeTravel(72)}
            className="flex items-center justify-center gap-1 bg-[#FF4E00] hover:bg-white hover:text-black text-black text-[10px] uppercase tracking-[0.15em] font-bold py-2 px-3 transition cursor-pointer"
          >
            <AlertTriangle className="w-3.5 h-3.5 text-black shrink-0" />
            Decay Attention
          </button>
        </div>
      </div>

      {/* Status Toasts */}
      {statusMessage.text && (
        <div
          className={`mb-4 px-3 py-2 text-xs flex items-center gap-2 rounded-none font-mono ${
            statusMessage.type === 'success'
              ? 'bg-[#FF4E00]/10 border border-[#FF4E00] text-[#FF4E00]'
              : statusMessage.type === 'info'
              ? 'bg-white/10 border border-white text-white'
              : 'bg-red-950 border border-red-800 text-red-200'
          }`}
        >
          <span className="font-bold uppercase text-[9px] bg-black/40 px-1.5 py-0.5 tracking-wider">
            {statusMessage.type}
          </span>
          <span className="tracking-wide">{statusMessage.text}</span>
        </div>
      )}

      {/* CDE PDF Drag simulation */}
      <div className="mb-6">
        <h3 className="text-[10px] uppercase tracking-[0.4em] text-white/40 mb-2">Commitment Discovery (CDE)</h3>
        <div className="flex gap-2">
          <input
            type="text"
            value={simulatedPdfName}
            onChange={(e) => setSimulatedPdfName(e.target.value)}
            className="bg-[#1A1A1A] border border-white/10 text-xs px-3 py-2 text-white flex-1 focus:outline-none focus:border-[#FF4E00] rounded-none font-mono"
          />
          <button
            onClick={handleDownloadSimulation}
            className="flex items-center justify-center bg-white hover:bg-[#FF4E00] text-black text-[10px] uppercase tracking-[0.2em] font-bold px-4 transition rounded-none cursor-pointer"
          >
            <Download className="w-3.5 h-3.5" />
          </button>
        </div>
        <p className="text-[10px] text-white/30 mt-1 leading-normal">Simulate reading PDF coursework and guidelines</p>
      </div>

      {/* Browser Telemetry Search simulation */}
      <div className="mb-6">
        <h3 className="text-[10px] uppercase tracking-[0.4em] text-white/40 mb-2">Browser Telemetry (Search Intercept)</h3>
        <div className="flex gap-2">
          <input
            type="text"
            placeholder="e.g. stripe signature verification Node failure"
            value={searchQueryInput}
            onChange={(e) => setSearchQueryInput(e.target.value)}
            onKeyDown={(e) => e.key === 'Enter' && handleSearchSimulation()}
            className="bg-[#1A1A1A] border border-white/10 text-xs px-3 py-2 text-white flex-1 focus:outline-none focus:border-[#FF4E00] rounded-none font-mono"
          />
          <button
            onClick={handleSearchSimulation}
            className="flex items-center justify-center bg-transparent hover:bg-white text-white hover:text-black text-[10px] uppercase tracking-[0.2em] font-bold px-4 border border-white/10 transition rounded-none cursor-pointer"
          >
            <Search className="w-3.5 h-3.5" />
          </button>
        </div>
        <p className="text-[10px] text-white/30 mt-1 leading-normal">Simulate search activity matching project queries</p>
      </div>

      {/* Submit Checkpoint */}
      <div className="border-t border-white/10 pt-5 mb-4">
        <h3 className="text-[10px] uppercase tracking-[0.4em] text-white/40 mb-3">Submit Ground-Truth Checkpoint</h3>
        <form onSubmit={handleCheckpointSubmit} className="space-y-4">
          <div>
            <label className="text-[9px] uppercase tracking-widest text-white/50 block mb-1.5 font-mono">Target Project</label>
            <select
              value={selectedProjectId}
              onChange={(e) => setSelectedProjectId(parseInt(e.target.value))}
              className="bg-[#1A1A1A] border border-white/10 text-xs px-3 py-2 text-white w-full focus:outline-none focus:border-[#FF4E00] rounded-none font-mono"
            >
              <option value={1}>ML Coursework Assignment</option>
              <option value={2}>Stripe Billing Integration</option>
            </select>
          </div>

          <div>
            <label className="text-[9px] uppercase tracking-widest text-white/50 block mb-1.5 font-mono">What was accomplished?</label>
            <textarea
              required
              placeholder="e.g. Built basic indexer notebook and processed guidelines."
              value={accomplished}
              onChange={(e) => setAccomplished(e.target.value)}
              className="bg-[#1A1A1A] border border-white/10 text-xs p-3 text-white w-full h-16 focus:outline-none focus:border-[#FF4E00] rounded-none resize-none font-sans"
            />
          </div>

          <div className="grid grid-cols-2 gap-3">
            <div>
              <label className="text-[9px] uppercase tracking-widest text-white/50 block mb-1.5 font-mono">What is blocked?</label>
              <input
                type="text"
                placeholder="e.g. Stuck on Stripe keys..."
                value={blocked}
                onChange={(e) => setBlocked(e.target.value)}
                className="bg-[#1A1A1A] border border-white/10 text-xs px-3 py-2 text-white w-full focus:outline-none focus:border-[#FF4E00] rounded-none"
              />
            </div>
            <div>
              <label className="text-[9px] uppercase tracking-widest text-white/50 block mb-1.5 font-mono">Next steps?</label>
              <input
                type="text"
                placeholder="e.g. Train local classifier..."
                value={nextSteps}
                onChange={(e) => setNextSteps(e.target.value)}
                className="bg-[#1A1A1A] border border-white/10 text-xs px-3 py-2 text-white w-full focus:outline-none focus:border-[#FF4E00] rounded-none"
              />
            </div>
          </div>

          <button
            type="submit"
            disabled={isSubmitting}
            className="w-full flex items-center justify-center gap-2 bg-[#FF4E00] hover:bg-white text-black hover:text-black text-[10px] uppercase tracking-[0.2em] font-black py-2.5 px-3 transition rounded-none cursor-pointer disabled:opacity-50"
          >
            <CheckSquare className="w-4 h-4" />
            {isSubmitting ? 'INDEXING_CORE...' : 'INDEX_CHECKPOINT_GROUND_TRUTH'}
          </button>
        </form>
      </div>

      {/* System Wipe / Privacy demo */}
      <div className="border-t border-white/10 pt-4 flex items-center justify-between">
        <span className="text-[9px] uppercase tracking-widest text-white/40 flex items-center gap-1 font-mono">
          <span className="w-1.5 h-1.5 bg-[#FF4E00] rounded-full"></span>
          OFFLINE PRIVACY SHIELD
        </span>
        <button
          onClick={handleSystemPurge}
          className="text-[9px] text-[#FF4E00] hover:text-white font-bold uppercase tracking-widest font-mono"
        >
          PURGE_TELEMETRY_DB
        </button>
      </div>
    </div>
  );
}
