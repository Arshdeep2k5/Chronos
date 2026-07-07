/**
 * @license
 * SPDX-License-Identifier: Apache-2.0
 */

import React, { useState, useEffect } from 'react';
import { API_BASE } from '../config';
import { Database, Search, ShieldCheck, Eye, Trash2 } from 'lucide-react';

interface DatabaseViewerProps {
  systemTime: string;
  refreshTrigger: number;
  onRefresh: () => void;
}

export default function DatabaseViewer({ systemTime, refreshTrigger, onRefresh }: DatabaseViewerProps) {
  const [selectedTable, setSelectedTable] = useState<string>('commitments');
  const [tableData, setTableData] = useState<{ [key: string]: any[] }>({});
  const [filterQuery, setFilterQuery] = useState('');
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    setLoading(true);
    fetch(`${API_BASE}/api/database`)
      .then(res => res.json())
      .then(data => {
        setTableData(data);
        setLoading(false);
      })
      .catch(e => {
        console.error(e);
        setLoading(false);
      });
  }, [refreshTrigger]);

  const tables = [
    { key: 'commitments', label: 'commitments (obligation ledger)' },
    { key: 'project_actions', label: 'project_actions (task queue)' },
    { key: 'context_nodes', label: 'context_nodes (tracked workspace files/domains)' },
    { key: 'context_events', label: 'context_events (interaction and focus times)' },
    { key: 'browser_sessions', label: 'browser_sessions (raw tab focus times)' },
    { key: 'search_queries', label: 'search_queries (browser search terms)' },
    { key: 'projects', label: 'projects (core projects)' },
    { key: 'workspace_snapshots', label: 'workspace_snapshots (IDE cursor positions)' },
    { key: 'dead_letter_queue', label: 'dead_letter_queue (failed parsing log)' }
  ];

  const currentRows = tableData[selectedTable] || [];

  const filteredRows = currentRows.filter(row => {
    const rowStr = JSON.stringify(row).toLowerCase();
    return rowStr.includes(filterQuery.toLowerCase());
  });

  return (
    <div id="database-viewer-panel" className="bg-[#0F0F0F] border border-white/10 p-6 shadow-2xl rounded-none">
      <div className="flex flex-col sm:flex-row sm:items-center justify-between border-b border-white/10 pb-4 mb-4 gap-2">
        <div>
          <h2 className="font-display font-black italic text-lg text-white flex items-center gap-2">
            <Database className="w-5 h-5 text-[#FF4E00]" />
            SOVEREIGN_TELEMETRY_LEDGER
          </h2>
          <p className="text-[10px] uppercase tracking-widest text-white/40 mt-1">Audit the local isolated sqlite transaction streams for complete sovereign compliance</p>
        </div>
        <div className="flex items-center gap-2 text-[10px] text-[#FF4E00] bg-white/5 border border-white/10 px-3 py-1 font-mono tracking-widest uppercase">
          <ShieldCheck className="w-4 h-4 text-[#FF4E00]" />
          <span>Local Sovereign Verification</span>
        </div>
      </div>

      {/* Selector dropdown and search */}
      <div className="flex flex-col sm:flex-row gap-4 mb-5">
        <div className="flex-1">
          <label className="text-[9px] font-bold text-white/40 uppercase tracking-widest block mb-1.5 font-mono">Select Active Ledger Table</label>
          <select
            value={selectedTable}
            onChange={(e) => setSelectedTable(e.target.value)}
            className="bg-[#1A1A1A] border border-white/10 text-xs px-3 py-2 text-white w-full focus:outline-none focus:border-[#FF4E00] rounded-none font-mono"
          >
            {tables.map(t => (
              <option key={t.key} value={t.key} className="bg-[#0F0F0F]">{t.label}</option>
            ))}
          </select>
        </div>
        <div className="sm:w-80">
          <label className="text-[9px] font-bold text-white/40 uppercase tracking-widest block mb-1.5 font-mono">Query Ledger Filter</label>
          <div className="relative">
            <input
              type="text"
              placeholder="Search sequence patterns..."
              value={filterQuery}
              onChange={(e) => setFilterQuery(e.target.value)}
              className="bg-[#1A1A1A] border border-white/10 text-xs pl-9 pr-3 py-2 text-white w-full focus:outline-none focus:border-[#FF4E00] rounded-none font-mono"
            />
            <Search className="w-3.5 h-3.5 text-white/30 absolute left-3 top-3.5" />
          </div>
        </div>
      </div>

      {/* Row Count Info */}
      <div className="flex items-center justify-between text-[10px] text-white/40 mb-2 font-mono uppercase tracking-wider">
        <span>Ledger Index Rows: {filteredRows.length} / {currentRows.length}</span>
        <span>Storage Format: Local JSON DB Vector Mode</span>
      </div>

      {/* Database log table display */}
      <div className="bg-black border border-white/10 overflow-x-auto max-h-80 select-text rounded-none">
        {loading ? (
          <div className="py-12 text-center text-xs text-white/30 font-mono animate-pulse">
            LOADING_SOVEREIGN_DATABASE_SCHEMA_LOGS...
          </div>
        ) : filteredRows.length === 0 ? (
          <div className="py-12 text-center text-xs text-white/30 font-mono">
            NO_ROWS_FOUND_MATCHING_CRITERIA
          </div>
        ) : (
          <table className="min-w-full divide-y divide-white/10 text-left font-mono text-[10px] text-white/70">
            <thead className="bg-[#121212] text-white/40 uppercase text-[9px] tracking-widest sticky top-0">
              <tr>
                {Object.keys(filteredRows[0]).map(key => (
                  <th key={key} className="px-4 py-3 border-b border-white/10 font-bold">{key}</th>
                ))}
              </tr>
            </thead>
            <tbody className="divide-y divide-white/5 bg-black/40">
              {filteredRows.map((row, rIdx) => (
                <tr key={rIdx} className="hover:bg-white/5 transition-colors">
                  {Object.values(row).map((val: any, vIdx) => (
                    <td key={vIdx} className="px-4 py-3 whitespace-nowrap max-w-xs truncate text-white/80">
                      {typeof val === 'object' && val !== null ? JSON.stringify(val) : String(val)}
                    </td>
                  ))}
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>

      {/* Privacy Guarantee footer */}
      <div className="bg-black p-4 border border-white/5 text-[10px] text-white/50 mt-4 leading-relaxed flex items-start gap-3 rounded-none">
        <Eye className="w-4 h-4 text-[#FF4E00] shrink-0 mt-0.5" />
        <div>
          <span className="font-mono uppercase text-white/80 font-bold block mb-1 tracking-wider">Sovereignty Shield Guarantee (Section 4.13)</span>
          Every local transaction sequence displayed in this dashboard exists strictly within the sandbox <code className="text-white font-bold font-mono">chronos_local_db.json</code>. Under no conditions is this dataset exposed to outside telemetry networks. Wiping or purging deletes records immediately.
        </div>
      </div>
    </div>
  );
}
