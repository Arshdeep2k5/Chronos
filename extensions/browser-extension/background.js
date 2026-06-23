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
    
    let urlString = tab.url || "";
    if (!urlString || urlString.startsWith("chrome://") || urlString.startsWith("about:")) return;
    
    let url;
    try {
      url = new URL(urlString);
    } catch (_) {
      return;
    }
    
    let domain = url.hostname;
    
    // Privacy Guardrails (Section 4.13 & Section 3.1.3)
    const blockedKeywords = ["login", "signin", "oauth", "password", "bank", "paypal", "checkout", "signup"];
    if (blockedKeywords.some(kw => domain.includes(kw) || url.pathname.includes(kw))) {
      console.log("Privacy Guard: Telemetry skipped for secure domain/URL");
      return;
    }
    
    sendTelemetry({
      type: "TAB_FOCUS",
      payload: {
        url: urlString,
        title: tab.title || "",
        domain: domain,
        visit_started_at: new Date().toISOString()
      }
    });
    
    let queryText = extractSearchQuery(url, tab.title || "");
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
