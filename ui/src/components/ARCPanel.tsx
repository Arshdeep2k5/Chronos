/**
 * @license
 * SPDX-License-Identifier: Apache-2.0
 */

import React, { useState, useEffect } from 'react';
import { API_BASE } from '../config';
import { Sparkles, Library, FileText, ArrowUpRight, Compass, Shield, Search } from 'lucide-react';
import { motion } from 'motion/react';

interface ResearchBrief {
  id: number;
  title: string;
  source: string;
  similarity: string;
  summary: string;
  link: string;
}

export default function ARCPanel() {
  const [briefs, setBriefs] = useState<ResearchBrief[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    fetch(`${API_BASE}/api/arc/briefs`)
      .then(res => res.json())
      .then(data => {
        setBriefs(data);
        setLoading(false);
      })
      .catch(e => console.error(e));
  }, []);

  return (
    <div id="arc-panel" className="bg-[#0F0F0F] border border-white/10 p-6 shadow-2xl rounded-none">
      <div className="flex items-center justify-between border-b border-white/10 pb-4 mb-4">
        <div>
          <h2 className="font-display font-black italic text-lg text-white flex items-center gap-2">
            <Sparkles className="w-4 h-4 text-[#FF4E00] animate-pulse" />
            AUTONOMOUS_RESEARCH_COMPANION
          </h2>
          <p className="text-[10px] uppercase tracking-widest text-white/40 mt-1">Smart background crawlers active during periods of user absence</p>
        </div>
        <span className="text-[9px] bg-white/5 border border-white/10 text-[#FF4E00] font-semibold px-2 py-0.5 rounded-none font-mono tracking-widest">
          ARC_ACTIVE
        </span>
      </div>

      <div className="bg-[#121212] border border-white/5 p-4 flex gap-3 mb-5 rounded-none">
        <Compass className="w-5 h-5 text-[#FF4E00] shrink-0 mt-0.5 animate-spin" style={{ animationDuration: '8s' }} />
        <div>
          <h4 className="text-[10px] uppercase tracking-[0.2em] text-[#FF4E00] font-bold">Autonomous Context Continuer</h4>
          <p className="text-[11px] text-white/50 mt-1.5 leading-relaxed">
            While your system was idle, Chronos traced your final active cognitive focus blockages and crawled index databases to pre-synthesize literature summaries.
          </p>
        </div>
      </div>

      {loading ? (
        <div className="py-8 text-center text-xs text-white/30 font-mono animate-pulse">
          CRAWLING_BIBLIOGRAPHIES_AND_REPOSITORIES...
        </div>
      ) : (
        <div className="space-y-4">
          {briefs.map((brief, idx) => (
            <motion.div
              key={brief.id}
              initial={{ opacity: 0, x: -10 }}
              animate={{ opacity: 1, x: 0 }}
              transition={{ duration: 0.3, delay: idx * 0.15 }}
              className="bg-black p-4 border border-white/10 hover:border-[#FF4E00]/40 transition-all duration-300 rounded-none"
            >
              <div className="flex items-start justify-between gap-3">
                <div>
                  <span className="text-[9px] font-mono font-bold text-black bg-[#FF4E00] px-2 py-0.5 uppercase tracking-wider">
                    {brief.similarity} MATCH
                  </span>
                  <h4 className="font-display font-bold text-sm text-white mt-2 leading-snug">
                    {brief.title}
                  </h4>
                  <div className="flex items-center gap-1.5 mt-1.5 text-[10px] text-white/40 font-mono uppercase tracking-wider">
                    <FileText className="w-3.5 h-3.5 text-white/20" />
                    Source: {brief.source}
                  </div>
                </div>
                <a
                  href={brief.link}
                  target="_blank"
                  rel="referrer"
                  className="bg-[#121212] hover:bg-white hover:text-black p-1.5 border border-white/10 text-white/50 transition cursor-pointer"
                >
                  <ArrowUpRight className="w-4 h-4" />
                </a>
              </div>

              <p className="text-xs text-white/60 mt-3 bg-[#121212] p-3 border border-white/5 leading-relaxed italic">
                "{brief.summary}"
              </p>
            </motion.div>
          ))}
        </div>
      )}

      {/* Local-first secure crawlers note */}
      <div className="mt-4 border-t border-white/10 pt-4 flex items-center gap-2 text-[10px] text-white/30 font-mono uppercase tracking-wider">
        <Shield className="w-3.5 h-3.5 text-[#FF4E00] shrink-0" />
        <span>Sovereign indexing engine. External documents parsed securely over loopback.</span>
      </div>
    </div>
  );
}
