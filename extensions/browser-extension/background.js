let ws = null;
let currentPort = 48120;
let authToken = "";
let activeTabId = null;
let activeTabStartTime = null;

chrome.storage.local.get(["auth_token", "port"], (data) => {
  authToken = data.auth_token || "default_token";
  currentPort = data.port || 48120;
  connectWS();
});

const ports = [48120, 48121, 48122, 48123];

function connectWS() {
  if (ws) {
    ws.close();
  }
  
  let connected = false;
  
  const tryConnect = (portIndex) => {
    if (portIndex >= ports.length) {
      console.log("Failed to connect to any daemon port.");
      setTimeout(() => connectWS(), 10000);
      return;
    }
    
    let port = ports[portIndex];
    let url = `ws://localhost:${port}/telemetry/browser?token=${authToken}`;
    let socket = new WebSocket(url);
    
    socket.onopen = () => {
      console.log(`Connected to daemon on port ${port}`);
      ws = socket;
      connected = true;
      currentPort = port;
      chrome.storage.local.set({ port: port });
    };
    
    socket.onmessage = (event) => {
      try {
        let data = JSON.parse(event.data);
        if (data.type === "DISTRACTION_INTERCEPT") {
          injectDopamineFrictionOverlay(activeTabId);
        }
      } catch(e) {
        console.error("Error parsing websocket message", e);
      }
    };

    socket.onerror = () => {
      if (!connected) {
        tryConnect(portIndex + 1);
      }
    };
    
    socket.onclose = () => {
      if (ws === socket) {
        ws = null;
        console.log("WebSocket connection closed. Reconnecting...");
        setTimeout(() => connectWS(), 5000);
      }
    };
  };
  
  tryConnect(0);
}

chrome.tabs.onActivated.addListener((activeInfo) => {
  handleTabFocusChange(activeInfo.tabId);
});

chrome.tabs.onUpdated.addListener((tabId, changeInfo, tab) => {
  if (changeInfo.status === 'complete' && tabId === activeTabId) {
    handleTabFocusChange(tabId);
  }
});

function handleTabFocusChange(tabId) {
  chrome.tabs.get(tabId, (tab) => {
    if (chrome.runtime.lastError || !tab) return;
    
    if (activeTabId && activeTabStartTime) {
      let duration = Math.round((Date.now() - activeTabStartTime) / 1000);
      // We can record focus duration if needed
    }
    
    activeTabId = tabId;
    activeTabStartTime = Date.now();
    
    let isPrivate = tab.incognito || false;
    let urlString = isPrivate ? "https://private.browsing/PrivateSession" : (tab.url || "");
    let tabTitle = isPrivate ? "Opened Private Window" : (tab.title || "");
    
    if (!urlString || urlString.startsWith("chrome://") || urlString.startsWith("about:")) return;
    
    let url;
    try {
      url = new URL(urlString);
    } catch (_) {
      return;
    }
    
    let domain = url.hostname;
    
    if (!isPrivate) {
      // Privacy Guardrails (Section 4.13 & Section 3.1.3)
      const blockedKeywords = ["login", "signin", "oauth", "password", "bank", "paypal", "checkout", "signup"];
      if (blockedKeywords.some(kw => domain.includes(kw) || url.pathname.includes(kw))) {
        console.log("Privacy Guard: Telemetry skipped for secure domain/URL");
        return;
      }
    }
    
    sendTelemetry({
      type: "TAB_FOCUS",
      payload: {
        url: urlString,
        title: tabTitle,
        domain: domain,
        visit_started_at: new Date().toISOString()
      }
    });
    
    if (!isPrivate) {
      let queryText = extractSearchQuery(url, tabTitle);
      if (queryText) {
        sendTelemetry({
          type: "SEARCH_QUERY",
          payload: {
            url: urlString,
            query_text: queryText,
            created_at: new Date().toISOString()
          }
        });
      }
    }
  });
}

function extractSearchQuery(url, title) {
  let hostname = url.hostname;
  let searchParams = url.searchParams;
  
  if (hostname.includes("google.") || hostname.includes("duckduckgo.") || hostname.includes("bing.")) {
    return searchParams.get("q");
  }
  if (hostname.includes("yahoo.")) {
    return searchParams.get("p");
  }
  if (hostname.includes("chatgpt.com") || hostname.includes("claude.ai") || hostname.includes("perplexity.ai")) {
    if (url.pathname.includes("/c/")) {
      return title;
    }
    return searchParams.get("q") || searchParams.get("query");
  }
  
  return null;
}

function sendTelemetry(msg) {
  if (ws && ws.readyState === WebSocket.OPEN) {
    ws.send(JSON.stringify(msg));
  }
}

function injectDopamineFrictionOverlay(tabId) {
  if (!tabId) return;
  chrome.scripting.executeScript({
    target: { tabId: tabId },
    func: () => {
      if (document.getElementById('chronos-dopamine-overlay')) return;
      
      const overlay = document.createElement('div');
      overlay.id = 'chronos-dopamine-overlay';
      overlay.style.position = 'fixed';
      overlay.style.top = '0';
      overlay.style.left = '0';
      overlay.style.width = '100vw';
      overlay.style.height = '100vh';
      overlay.style.backgroundColor = 'rgba(0, 0, 0, 0.9)';
      overlay.style.color = '#fff';
      overlay.style.zIndex = '2147483647';
      overlay.style.display = 'flex';
      overlay.style.flexDirection = 'column';
      overlay.style.justifyContent = 'center';
      overlay.style.alignItems = 'center';
      overlay.style.fontFamily = 'monospace';
      overlay.style.fontSize = '24px';
      
      const text = document.createElement('div');
      text.innerText = 'Chronos Pilot: Dopamine Friction Triggered';
      text.style.marginBottom = '20px';
      text.style.color = '#f44336';
      
      const subText = document.createElement('div');
      subText.innerText = 'You have an imminent commitment due in < 48 hours.';
      subText.style.fontSize = '18px';
      subText.style.marginBottom = '40px';
      
      const countdown = document.createElement('div');
      countdown.innerText = '5';
      countdown.style.fontSize = '48px';
      countdown.style.fontWeight = 'bold';
      
      overlay.appendChild(text);
      overlay.appendChild(subText);
      overlay.appendChild(countdown);
      document.body.appendChild(overlay);
      
      let seconds = 5;
      const interval = setInterval(() => {
        seconds--;
        countdown.innerText = seconds.toString();
        if (seconds <= 0) {
          clearInterval(interval);
          overlay.remove();
        }
      }, 1000);
    }
  });
}
