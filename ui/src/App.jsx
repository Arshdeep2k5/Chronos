import { createSignal, createEffect } from 'solid-js';

function App() {
  const [activeScene, setActiveScene] = createSignal(0);
  const [dragOver, setDragOver] = createSignal(false);
  const [isRestored, setIsRestored] = createSignal(false);
  const [diagnostics, setDiagnostics] = createSignal(null);
  const [trajectory, setTrajectory] = createSignal(null);

  createEffect(() => {
    if (activeScene() === 2) {
      fetch('http://localhost:48120/api/trajectory')
        .then(res => res.json())
        .then(data => setTrajectory(data))
        .catch(err => console.error("Failed to fetch trajectory:", err));
    }
  });

  const handleRestoreWorkspace = async () => {
    setIsRestored(true);
    try {
      // Trigger IDE restore
      await fetch('http://localhost:48120/api/restore', { method: 'POST' });
      
      // Fetch diagnostics
      const res = await fetch('http://localhost:48120/api/diagnostics');
      if (res.ok) {
        const data = await res.json();
        setDiagnostics(data);
      }
    } catch (e) {
      console.error("Failed to restore or fetch diagnostics", e);
    }
  };

  const handlePrivacyWipe = async () => {
    if (confirm("Are you sure you want to wipe all Chronos telemetry history?")) {
      try {
        await fetch('http://localhost:48120/api/privacy/wipe', { method: 'DELETE' });
        alert("Telemetry history wiped successfully.");
      } catch (e) {
        console.error("Failed to wipe history", e);
      }
    }
  };

  const getHealthColor = (level) => {
    if (level === 'CRITICAL' || level === 'HIGH') return '#ef4444'; // Red
    if (level === 'MEDIUM') return '#f59e0b'; // Yellow
    return '#10b981'; // Green
  };

  const getHealthBg = (level) => {
    if (level === 'CRITICAL' || level === 'HIGH') return 'rgba(239, 68, 68, 0.05)';
    if (level === 'MEDIUM') return 'rgba(245, 158, 11, 0.05)';
    return 'rgba(16, 185, 129, 0.05)';
  };

  const getHealthBorder = (level) => {
    if (level === 'CRITICAL' || level === 'HIGH') return '1px solid rgba(239, 68, 68, 0.2)';
    if (level === 'MEDIUM') return '1px solid rgba(245, 158, 11, 0.2)';
    return '1px solid rgba(16, 185, 129, 0.2)';
  };

  const getHealthLabel = (level) => {
    if (level === 'CRITICAL' || level === 'HIGH') return 'CRITICAL RISK ALERT';
    if (level === 'MEDIUM') return 'WARNING: TIGHT SCHEDULE';
    return 'ON TRACK';
  };

  const getHealthTitle = (level) => {
    if (level === 'CRITICAL' || level === 'HIGH') return 'Deadline Threat Detected';
    if (level === 'MEDIUM') return 'Health Engine: Moderate Risk';
    return 'Project Healthy';
  };

  // Demo state data
  const scenes = [
    { name: "1. Commitment Discovery", desc: "Ingesting coursework PDF and watching HUD extract deadline." },
    { name: "2. Web Focus Tracking", desc: "Browser Telemetry Connector tracking active focus sessions." },
    { name: "3. Trajectory Risk Forecast", desc: "Manually fast-forwarding time to show failure forecasting." },
    { name: "4. Recovery Planning", desc: "Generating catch-up schedule segments." },
    { name: "5. One-Click Restore", desc: "Launching editor, restoring browser tabs, and displaying diagnostics." },
    { name: "6. Autonomous Research", desc: "Background researcher presenting briefs during user absence." },
    { name: "7. Checkpoints & Search", desc: "Tier 3: Semantic context search and human checkpoints." }
  ];

  return (
    <div style={{
      display: 'flex',
      height: '100vh',
      background: 'radial-gradient(circle at 10% 20%, rgb(18, 20, 26) 0%, rgb(10, 10, 14) 90%)',
      color: '#f3f4f6',
      'font-family': 'var(--font-sans)',
      overflow: 'hidden'
    }}>
      
      {/* Sidebar Control Deck for Hackathon Presenter */}
      <div style={{
        width: '320px',
        borderRight: '1px solid var(--border-glass)',
        padding: '24px',
        background: 'rgba(10, 12, 16, 0.9)',
        display: 'flex',
        flexDirection: 'column',
        gap: '20px'
      }}>
        <div>
          <h1 id="deck-title" style={{ 'font-size': '22px', 'font-weight': '800', 'letter-spacing': '-0.5px', color: '#818cf8', 'margin-bottom': '4px' }}>Chronos Pilot</h1>
          <p style={{ 'font-size': '12px', color: 'var(--text-secondary)' }}>V1.0 HACKATHON CONTROL DECK</p>
        </div>

        <div style={{ display: 'flex', flexDirection: 'column', gap: '8px', flex: 1 }}>
          {scenes.map((scene, idx) => (
            <button
              id={`scene-btn-${idx}`}
              onClick={() => setActiveScene(idx)}
              style={{
                width: '100%',
                padding: '12px',
                'border-radius': '10px',
                border: activeScene() === idx ? '1px solid #6366f1' : '1px solid rgba(255,255,255,0.05)',
                background: activeScene() === idx ? 'rgba(99, 102, 241, 0.15)' : 'rgba(255,255,255,0.02)',
                color: activeScene() === idx ? '#fff' : 'var(--text-secondary)',
                cursor: 'pointer',
                textAlign: 'left',
                transition: 'var(--transition-smooth)'
              }}
            >
              <div style={{ 'font-weight': '600', 'font-size': '13px' }}>{scene.name}</div>
              <div style={{ 'font-size': '11px', 'margin-top': '4px', opacity: 0.7 }}>{scene.desc}</div>
            </button>
          ))}
        </div>

        <div style={{ borderTop: '1px solid rgba(255,255,255,0.05)', 'padding-top': '16px', display: 'flex', flexDirection: 'column', gap: '12px' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
            <div style={{ width: '8px', height: '8px', borderRadius: '50%', background: '#10b981' }}></div>
            <span style={{ 'font-size': '12px', color: 'var(--text-secondary)' }}>Daemon: PORT 48120</span>
          </div>
          <button 
            onClick={handlePrivacyWipe}
            style={{
              padding: '8px',
              background: 'rgba(239, 68, 68, 0.1)',
              border: '1px solid rgba(239, 68, 68, 0.2)',
              color: '#ef4444',
              borderRadius: '8px',
              cursor: 'pointer',
              fontSize: '12px',
              fontWeight: '600'
            }}
          >
            Wipe Telemetry History
          </button>
        </div>
      </div>

      {/* Main HUD Window */}
      <div style={{ flex: 1, padding: '40px', overflowY: 'auto', display: 'flex', flexDirection: 'column', gap: '24px' }}>
        
        {/* Scene 1: Commitment Discovery */}
        {activeScene() === 0 && (
          <div style={{ display: 'flex', flexDirection: 'column', gap: '20px' }}>
            <h2 style={{ 'font-size': '24px', 'font-weight': '800' }}>Zero-Configuration Commitment Discovery</h2>
            
            <div
              id="drop-zone"
              onDragOver={(e) => { e.preventDefault(); setDragOver(true); }}
              onDragLeave={() => setDragOver(false)}
              onDrop={(e) => { e.preventDefault(); setDragOver(false); }}
              style={{
                border: dragOver() ? '2px dashed #6366f1' : '2px dashed rgba(255,255,255,0.1)',
                padding: '60px',
                'border-radius': '16px',
                background: dragOver() ? 'rgba(99, 102, 241, 0.05)' : 'rgba(255,255,255,0.02)',
                textAlign: 'center',
                cursor: 'pointer',
                transition: 'var(--transition-smooth)'
              }}
            >
              <p style={{ 'font-size': '15px', 'margin-bottom': '12px' }}>Drag and Drop your assignment file here (e.g. <code>ML_Assignment.pdf</code>)</p>
              <span style={{ 'font-size': '12px', color: 'var(--text-secondary)' }}>Monitored path: ~/Downloads</span>
            </div>

            {/* Extracted Commitment Card */}
            <div style={{
              background: 'rgba(20, 24, 33, 0.6)',
              border: '1px solid var(--border-glass)',
              padding: '20px',
              'border-radius': '16px',
              backdropFilter: 'blur(20px)'
            }}>
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                <div>
                  <span style={{ 'font-size': '11px', background: '#312e81', color: '#c7d2fe', padding: '4px 8px', 'border-radius': '12px', 'font-weight': '600' }}>ASSIGNMENT DISCOVERED</span>
                  <h3 style={{ 'font-size': '18px', 'font-weight': '700', 'margin-top': '8px' }}>ML Assignment</h3>
                  <p style={{ 'font-size': '13px', color: 'var(--text-secondary)', 'margin-top': '4px' }}>Source: <code>~/Downloads/ML_Assignment.pdf</code></p>
                </div>
                <div style={{ textAlign: 'right' }}>
                  <div style={{ 'font-size': '13px', color: 'var(--text-secondary)' }}>Confidence</div>
                  <div style={{ 'font-size': '24px', 'font-weight': '800', color: '#10b981' }}>92%</div>
                </div>
              </div>
              <div style={{ borderTop: '1px solid rgba(255,255,255,0.05)', 'margin-top': '16px', 'padding-top': '16px', display: 'flex', gap: '40px' }}>
                <div>
                  <div style={{ 'font-size': '12px', color: 'var(--text-secondary)' }}>EXTRACTED DEADLINE</div>
                  <div style={{ 'font-weight': '600', 'margin-top': '4px' }}>July 15, 2026</div>
                </div>
                <div>
                  <div style={{ 'font-size': '12px', color: 'var(--text-secondary)' }}>REMAINING TIME</div>
                  <div style={{ 'font-weight': '600', 'margin-top': '4px', color: '#f59e0b' }}>22 days remaining</div>
                </div>
              </div>
            </div>
          </div>
        )}

        {/* Scene 2: Web Research Focus Tracking */}
        {activeScene() === 1 && (
          <div style={{ display: 'flex', flexDirection: 'column', gap: '20px' }}>
            <h2 style={{ 'font-size': '24px', 'font-weight': '800' }}>Proactive Focus Tracking & Web Research</h2>
            
            <div style={{
              background: 'rgba(20, 24, 33, 0.6)',
              border: '1px solid var(--border-glass)',
              padding: '24px',
              'border-radius': '16px',
              display: 'flex',
              flexDirection: 'column',
              gap: '16px'
            }}>
              <h3 style={{ 'font-size': '16px', 'font-weight': '600' }}>Active Research Session: RAG Chunking</h3>
              
              <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
                <div style={{ display: 'flex', justifyContent: 'space-between', padding: '12px', background: 'rgba(255,255,255,0.02)', borderRadius: '8px' }}>
                  <div>
                    <div style={{ 'font-size': '13px', 'font-weight': '600' }}>Google Search: "RAG chunking strategy embeddings example"</div>
                    <div style={{ 'font-size': '11px', color: 'var(--text-secondary)', 'margin-top': '2px' }}>Focused for 15 seconds</div>
                  </div>
                  <span style={{ color: '#10b981', 'font-size': '12px', 'align-self': 'center' }}>Active Now</span>
                </div>
                <div style={{ display: 'flex', justifyContent: 'space-between', padding: '12px', background: 'rgba(255,255,255,0.01)', borderRadius: '8px' }}>
                  <div>
                    <div style={{ 'font-size': '13px', 'font-weight': '600' }}>Pinecone Docs: Chunking Strategies for Vector Databases</div>
                    <div style={{ 'font-size': '11px', color: 'var(--text-secondary)', 'margin-top': '2px' }}>Visited 2 minutes ago</div>
                  </div>
                  <span style={{ color: 'var(--text-secondary)', 'font-size': '12px', 'align-self': 'center' }}>3m focus</span>
                </div>
              </div>
            </div>
          </div>
        )}

        {/* Scene 3: Trajectory Risk Forecast */}
        {activeScene() === 2 && (
          <div style={{ display: 'flex', flexDirection: 'column', gap: '20px' }}>
            <h2 style={{ 'font-size': '24px', 'font-weight': '800' }}>Time Jump & Risk Forecasting</h2>
            
            <div style={{
              background: getHealthBg(trajectory()?.risk_level),
              border: getHealthBorder(trajectory()?.risk_level),
              padding: '24px',
              'border-radius': '16px',
              display: 'flex',
              flexDirection: 'column',
              gap: '20px'
            }}>
              <div style={{ display: 'flex', justifyContent: 'space-between' }}>
                <div>
                  <span style={{ 
                    background: getHealthColor(trajectory()?.risk_level), 
                    color: '#fff', 'font-size': '11px', padding: '4px 8px', 'border-radius': '12px', 'font-weight': '700' 
                  }}>
                    {getHealthLabel(trajectory()?.risk_level)}
                  </span>
                  <h3 style={{ 
                    'font-size': '20px', 'font-weight': '800', 'margin-top': '8px', 
                    color: getHealthColor(trajectory()?.risk_level) 
                  }}>
                    {getHealthTitle(trajectory()?.risk_level)}
                  </h3>
                </div>
                <div style={{ textAlign: 'right' }}>
                  <div style={{ 'font-size': '12px', color: 'var(--text-secondary)' }}>Risk Score</div>
                  <div style={{ 
                    'font-size': '28px', 'font-weight': '800', 
                    color: getHealthColor(trajectory()?.risk_level) 
                  }}>
                    {trajectory()?.risk_score || 0.81} ({trajectory()?.risk_level || 'HIGH'})
                  </div>
                </div>
              </div>

              <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '20px' }}>
                <div style={{ background: 'rgba(255,255,255,0.02)', padding: '16px', borderRadius: '12px' }}>
                  <div style={{ 'font-size': '12px', color: 'var(--text-secondary)' }}>COMPLETION PROBABILITY</div>
                  <div style={{ 'font-size': '24px', 'font-weight': '800', 'margin-top': '4px' }}>
                    {trajectory()?.completion_probability || 41}%
                  </div>
                </div>
                <div style={{ background: 'rgba(255,255,255,0.02)', padding: '16px', borderRadius: '12px' }}>
                  <div style={{ 'font-size': '12px', color: 'var(--text-secondary)' }}>ESTIMATED COGNITIVE DECAY</div>
                  <div style={{ 'font-size': '24px', 'font-weight': '800', 'margin-top': '4px', color: '#f59e0b' }}>
                    {trajectory()?.cognitive_decay_date || 'July 12'}
                  </div>
                </div>
              </div>

              <div style={{ borderTop: '1px solid rgba(255,255,255,0.05)', 'padding-top': '16px' }}>
                <h4 style={{ 'font-size': '13px', 'font-weight': '600', 'margin-bottom': '8px' }}>CONSEQUENCE SIMULATOR</h4>
                <p style={{ 'font-size': '13px', color: 'var(--text-secondary)' }}>
                  {trajectory()?.simulator_message || '⚠️ Postponing action for another 24h will drop the completion probability to 28% (a marginal drop of -13%).'}
                </p>
              </div>

              {trajectory()?.why_now && trajectory().why_now.length > 0 && (
                <div style={{ borderTop: '1px solid rgba(255,255,255,0.05)', 'padding-top': '16px' }}>
                  <h4 style={{ 'font-size': '13px', 'font-weight': '600', 'margin-bottom': '8px', color: '#f87171' }}>WHY?</h4>
                  <ul style={{ 'font-size': '13px', color: 'var(--text-secondary)', 'padding-left': '20px', 'margin': 0 }}>
                    {trajectory().why_now.map((reason, i) => (
                      <li key={i} style={{ 'margin-bottom': '4px' }}>{reason}</li>
                    ))}
                  </ul>
                </div>
              )}
            </div>
          </div>
        )}

        {/* Scene 4: Recovery Planning Synthesis */}
        {activeScene() === 3 && (
          <div style={{ display: 'flex', flexDirection: 'column', gap: '20px' }}>
            <h2 style={{ 'font-size': '24px', 'font-weight': '800' }}>Recovery Planning Synthesis</h2>
            
            <div style={{
              background: 'var(--bg-surface)',
              border: '1px solid var(--border-glass)',
              padding: '24px',
              'border-radius': '16px'
            }}>
              <h3 style={{ 'font-size': '16px', 'font-weight': '700', 'margin-bottom': '16px' }}>RECOVERY PLAN: ML Assignment</h3>
              
              <div style={{ display: 'flex', flexDirection: 'column', gap: '12px' }}>
                <div style={{ display: 'flex', gap: '12px', alignItems: 'center', padding: '12px', background: 'rgba(255,255,255,0.02)', borderRadius: '8px' }}>
                  <input type="checkbox" checked={false} style={{ width: '18px', height: '18px', cursor: 'pointer' }} />
                  <div>
                    <span style={{ 'font-size': '13px', 'font-weight': '600' }}>[Today] - Complete literature review & parse inputs</span>
                    <span style={{ 'font-size': '11px', color: '#818cf8', 'margin-left': '12px' }}>(3.5h effort)</span>
                  </div>
                </div>
                <div style={{ display: 'flex', gap: '12px', alignItems: 'center', padding: '12px', background: 'rgba(255,255,255,0.02)', borderRadius: '8px' }}>
                  <input type="checkbox" checked={false} style={{ width: '18px', height: '18px', cursor: 'pointer' }} />
                  <div>
                    <span style={{ 'font-size': '13px', 'font-weight': '600' }}>[Tomorrow] - Train local classifier & run baseline</span>
                    <span style={{ 'font-size': '11px', color: '#818cf8', 'margin-left': '12px' }}>(4.0h effort)</span>
                  </div>
                </div>
                <div style={{ display: 'flex', gap: '12px', alignItems: 'center', padding: '12px', background: 'rgba(255,255,255,0.02)', borderRadius: '8px' }}>
                  <input type="checkbox" checked={false} style={{ width: '18px', height: '18px', cursor: 'pointer' }} />
                  <div>
                    <span style={{ 'font-size': '13px', 'font-weight': '600' }}>[Wednesday] - Finalize report writing & export PDFs</span>
                    <span style={{ 'font-size': '11px', color: '#818cf8', 'margin-left': '12px' }}>(2.5h effort)</span>
                  </div>
                </div>
              </div>
            </div>
          </div>
        )}

        {/* Scene 5: One-Click Restore */}
        {activeScene() === 4 && (
          <div style={{ display: 'flex', flexDirection: 'column', gap: '20px' }}>
            <h2 style={{ 'font-size': '24px', 'font-weight': '800' }}>One-Click Restoration & "Why You Stopped"</h2>
            
            <button
              id="start-working-btn"
              onClick={handleRestoreWorkspace}
              style={{
                padding: '16px 24px',
                background: '#6366f1',
                color: '#fff',
                border: 'none',
                'border-radius': '12px',
                'font-size': '16px',
                'font-weight': '700',
                cursor: 'pointer',
                boxShadow: '0 4px 14px var(--accent-glow)',
                transition: 'var(--transition-smooth)',
                width: 'fit-content'
              }}
            >
              Start Working
            </button>

            {isRestored() && (
              <div style={{
                background: 'rgba(20, 24, 33, 0.7)',
                border: '1px solid var(--border-glass)',
                padding: '24px',
                'border-radius': '16px',
                display: 'flex',
                flexDirection: 'column',
                gap: '16px',
                animation: 'fadeIn 0.5s ease-out'
              }}>
                <h3 style={{ 'font-size': '16px', 'font-weight': '700', color: '#818cf8' }}>-- STOPPING-POINT DIAGNOSTICS --</h3>
                
                <div style={{ display: 'flex', flexDirection: 'column', gap: '10px', 'font-size': '13px' }}>
                  <p><strong>Abandonment Trigger:</strong> System idle detected.</p>
                  <div style={{ padding: '12px', background: 'rgba(0,0,0,0.2)', borderRadius: '8px', borderLeft: '3px solid #f59e0b' }}>
                    <p><strong>Final open file:</strong> <code>{diagnostics()?.active_file || 'billing/stripe_webhook.ts'}</code> (Line {diagnostics()?.cursor_line || 42})</p>
                    <p style={{ 'margin-top': '4px' }}><strong>Final search query:</strong> "{diagnostics()?.last_search || 'stripe webhook SignatureVerificationException local testing'}"</p>
                    <p style={{ 'margin-top': '4px' }}><strong>Last checkpoint blocker:</strong> "{diagnostics()?.blocker || 'Stuck trying to mock Stripe signatures locally.'}"</p>
                  </div>
                  
                  {diagnostics()?.narrative && (
                    <div style={{ 'margin-top': '12px', padding: '12px', background: 'rgba(255,255,255,0.05)', borderRadius: '8px' }}>
                      <h4 style={{ 'font-size': '12px', 'margin-bottom': '8px', color: '#c7d2fe' }}>NARRATIVE RECONSTRUCTION</h4>
                      <div style={{ 'font-size': '12px', 'white-space': 'pre-wrap', color: 'var(--text-secondary)' }}>
                        {diagnostics().narrative}
                      </div>
                    </div>
                  )}
                </div>
              </div>
            )}
          </div>
        )}

        {/* Scene 6: Autonomous Research Brief */}
        {activeScene() === 5 && (
          <div style={{ display: 'flex', flexDirection: 'column', gap: '20px' }}>
            <h2 style={{ 'font-size': '24px', 'font-weight': '800' }}>Autonomous Research Brief</h2>
            
            <div style={{
              background: 'var(--bg-surface)',
              border: '1px solid var(--border-glass)',
              padding: '24px',
              'border-radius': '16px'
            }}>
              <h3 style={{ 'font-size': '16px', 'font-weight': '700', 'margin-bottom': '16px' }}>While You Were Away - References Gathered</h3>
              
              <div style={{ display: 'flex', flexDirection: 'column', gap: '16px' }}>
                <div style={{ padding: '16px', background: 'rgba(255,255,255,0.02)', borderRadius: '12px' }}>
                  <div style={{ display: 'flex', justifyContent: 'space-between' }}>
                     <span style={{ 'font-size': '11px', background: '#065f46', color: '#a7f3d0', padding: '2px 8px', borderRadius: '10px', 'font-weight': '600' }}>arXiv Research</span>
                    <span style={{ 'font-size': '12px', color: '#10b981', 'font-weight': '600' }}>Match Score: 85%</span>
                  </div>
                  <h4 style={{ 'font-size': '14px', 'font-weight': '600', 'margin-top': '8px' }}>"Chunking Strategies for Large Language Model RAG Ingestion Pipeline"</h4>
                  <p style={{ 'font-size': '12px', color: 'var(--text-secondary)', 'margin-top': '4px' }}>
                    Analyzes dynamic document segmentation boundaries using paragraph overlaps and syntax trees to optimize context window embeddings retrieval.
                  </p>
                </div>

                <div style={{ padding: '16px', background: 'rgba(255,255,255,0.02)', borderRadius: '12px' }}>
                  <div style={{ display: 'flex', justifyContent: 'space-between' }}>
                    <span style={{ 'font-size': '11px', background: '#1e3a8a', color: '#dbeafe', padding: '2px 8px', borderRadius: '10px', 'font-weight': '600' }}>GitHub Repo</span>
                    <span style={{ 'font-size': '12px', color: '#10b981', 'font-weight': '600' }}>Match Score: 80%</span>
                  </div>
                  <h4 style={{ 'font-size': '14px', 'font-weight': '600', 'margin-top': '8px' }}>langchain-ai/rag-chunkers</h4>
                  <p style={{ 'font-size': '12px', color: 'var(--text-secondary)', 'margin-top': '4px' }}>
                    Standard library implementation offering semantic text splitters, character boundary counters, and tokenizers for major LLM provider models.
                  </p>
                </div>
              </div>
            </div>
          </div>
        )}

        {/* Scene 7: Human Checkpoints & Smart Search */}
        {activeScene() === 6 && (
          <div style={{ display: 'flex', flexDirection: 'column', gap: '20px' }}>
            <h2 style={{ 'font-size': '24px', 'font-weight': '800' }}>Tier 3: Checkpoints & Search</h2>
            
            {/* Smart Search */}
            <div style={{
              background: 'var(--bg-surface)',
              border: '1px solid var(--border-glass)',
              padding: '24px',
              'border-radius': '16px'
            }}>
              <h3 style={{ 'font-size': '16px', 'font-weight': '700', 'margin-bottom': '16px' }}>Cognitive Context Search</h3>
              <div style={{ display: 'flex', gap: '12px' }}>
                <input 
                  id="smart-search-input"
                  type="text" 
                  placeholder="Search your workflow history..."
                  style={{
                    flex: 1, padding: '12px', borderRadius: '8px', border: '1px solid rgba(255,255,255,0.1)',
                    background: 'rgba(0,0,0,0.2)', color: '#fff', outline: 'none'
                  }}
                />
                <button 
                  onClick={() => {
                    const q = document.getElementById('smart-search-input').value;
                    fetch(`http://localhost:48120/api/search?q=${encodeURIComponent(q)}`)
                      .then(res => res.json())
                      .then(data => alert(`Found ${data.length} results. Top result: ` + (data[0] ? data[0].display_name : 'None')));
                  }}
                  style={{ padding: '0 20px', background: '#3b82f6', color: '#fff', border: 'none', borderRadius: '8px', cursor: 'pointer', fontWeight: '600' }}>Search</button>
              </div>
            </div>

            {/* Human Checkpoint */}
            <div style={{
              background: 'var(--bg-surface)',
              border: '1px solid var(--border-glass)',
              padding: '24px',
              'border-radius': '16px'
            }}>
              <h3 style={{ 'font-size': '16px', 'font-weight': '700', 'margin-bottom': '16px' }}>Submit Human Checkpoint</h3>
              <div style={{ display: 'flex', flexDirection: 'column', gap: '12px' }}>
                <textarea id="chk-acc" placeholder="What was accomplished?" style={{ padding: '12px', borderRadius: '8px', border: '1px solid rgba(255,255,255,0.1)', background: 'rgba(0,0,0,0.2)', color: '#fff', outline: 'none' }} rows="2"></textarea>
                <textarea id="chk-blk" placeholder="What is blocking you?" style={{ padding: '12px', borderRadius: '8px', border: '1px solid rgba(255,255,255,0.1)', background: 'rgba(0,0,0,0.2)', color: '#fff', outline: 'none' }} rows="2"></textarea>
                <textarea id="chk-nxt" placeholder="Next steps?" style={{ padding: '12px', borderRadius: '8px', border: '1px solid rgba(255,255,255,0.1)', background: 'rgba(0,0,0,0.2)', color: '#fff', outline: 'none' }} rows="2"></textarea>
                <button 
                  onClick={() => {
                    fetch('http://localhost:48120/api/checkpoints', {
                      method: 'POST',
                      headers: {'Content-Type': 'application/json'},
                      body: JSON.stringify({
                        accomplished: document.getElementById('chk-acc').value,
                        blocked: document.getElementById('chk-blk').value,
                        next_steps: document.getElementById('chk-nxt').value,
                      })
                    }).then(() => alert('Checkpoint Saved!'));
                  }}
                  style={{ padding: '12px', background: '#10b981', color: '#fff', border: 'none', borderRadius: '8px', cursor: 'pointer', fontWeight: '600', marginTop: '8px' }}>
                  Save Checkpoint
                </button>
              </div>
            </div>

          </div>
        )}

      </div>
    </div>
  );
}

export default App;
