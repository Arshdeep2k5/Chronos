const vscode = require('vscode');
const WebSocket = require('ws');
const fs = require('fs');
const path = require('path');
const os = require('os');

let socket = null;
let reconnectTimeout = null;

function getHandshake() {
  const home = os.homedir();
  const handshakePath = path.join(home, '.config', 'chronos', 'handshake.json');
  try {
    if (fs.existsSync(handshakePath)) {
      const data = fs.readFileSync(handshakePath, 'utf8');
      return JSON.parse(data);
    }
  } catch (e) {
    console.error("Error reading handshake.json:", e);
  }
  return { auth_token: "default_token", port: 48120 };
}

function activate(context) {
  console.log('Chronos VSCode Connector is active.');
  
  connectWS();
  
  // Register telemetry event listeners
  const activeEditorChangeDisposable = vscode.window.onDidChangeActiveTextEditor(() => {
    sendTelemetry();
  });
  
  const selectionChangeDisposable = vscode.window.onDidChangeTextEditorSelection((e) => {
    if (e.textEditor === vscode.window.activeTextEditor) {
      sendTelemetry();
    }
  });
  
  context.subscriptions.push(activeEditorChangeDisposable);
  context.subscriptions.push(selectionChangeDisposable);
}

function connectWS() {
  if (socket) {
    socket.close();
  }
  
  const { auth_token, port } = getHandshake();
  const url = `ws://localhost:${port}/telemetry/ide?token=${auth_token}`;
  
  socket = new WebSocket(url);
  
  socket.on('open', () => {
    console.log('Connected to Chronos Daemon.');
    sendTelemetry();
  });
  
  socket.on('message', (data) => {
    try {
      const message = JSON.parse(data.toString());
      if (message.type === 'WORKSPACE_RESTORE') {
        handleRestore(message.payload);
      }
    } catch (e) {
      console.error('Error processing socket message:', e);
    }
  });
  
  socket.on('error', (err) => {
    console.error('Socket error:', err.message);
  });
  
  socket.on('close', () => {
    console.log('Connection closed. Retrying...');
    if (reconnectTimeout) {
      clearTimeout(reconnectTimeout);
    }
    reconnectTimeout = setTimeout(connectWS, 5000);
  });
}

function sendTelemetry() {
  if (!socket || socket.readyState !== WebSocket.OPEN) {
    return;
  }
  
  const editor = vscode.window.activeTextEditor;
  if (!editor) return;
  
  const tabs = [];
  try {
    for (const group of vscode.window.tabGroups.all) {
      for (const tab of group.tabs) {
        if (tab.input && tab.input.uri) {
          tabs.push(tab.input.uri.fsPath);
        }
      }
    }
  } catch (_) {}
  
  const payload = {
    active_file_path: editor.document.uri.fsPath,
    cursor_line: editor.selection.active.line + 1,
    cursor_column: editor.selection.active.character + 1,
    open_tabs: tabs,
    timestamp: new Date().toISOString()
  };
  
  socket.send(JSON.stringify({
    type: 'WORKSPACE_TELEMETRY',
    payload: payload
  }));
}

function handleRestore(payload) {
  // Restore all tabs in layout
  const openTabPromises = (payload.open_tabs || []).map(filePath => {
    return vscode.workspace.openTextDocument(vscode.Uri.file(filePath))
      .then(doc => vscode.window.showTextDocument(doc, { preview: false }));
  });
  
  Promise.all(openTabPromises).then(() => {
    if (payload.active_file_path) {
      vscode.workspace.openTextDocument(vscode.Uri.file(payload.active_file_path)).then(doc => {
        vscode.window.showTextDocument(doc, { preview: false }).then(editor => {
          const position = new vscode.Position(payload.cursor_line - 1, payload.cursor_column - 1);
          editor.selection = new vscode.Selection(position, position);
          editor.revealRange(new vscode.Range(position, position));
        });
      });
    }
  });
}

function deactivate() {
  if (socket) {
    socket.close();
  }
  if (reconnectTimeout) {
    clearTimeout(reconnectTimeout);
  }
}

module.exports = {
  activate,
  deactivate
};
