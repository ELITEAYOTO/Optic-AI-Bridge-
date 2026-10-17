'use strict';
// Trusted operator viewer. AI sources are renderer assets, never Electron main/preload code.
const {app, BrowserWindow, protocol, session} = require('electron');
const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const assert = require('node:assert/strict');
const policy = require('./viewer_policy.cjs');
const args = process.argv.slice(2);
const repeat = args.length === 3 && args[2] === '--smoke-repeat';
const smoke = args.length === 3 && (args[2] === '--smoke' || repeat);
if (args.length !== 2 && !smoke) throw new Error('viewer.invalid_arguments');
const snapshot = policy.loadSnapshot(args[0], args[1]);
const data = path.join(snapshot.root, 'electron-data');
if (fs.existsSync(data) && (!fs.lstatSync(data).isDirectory() || fs.lstatSync(data).isSymbolicLink())) throw new Error('viewer.invalid_data');
fs.mkdirSync(data, {recursive: true});
app.setPath('userData', data);
app.disableHardwareAcceleration();
app.commandLine.appendSwitch('disable-background-networking');
protocol.registerSchemesAsPrivileged([{scheme: 'optic-alpha', privileges: {standard: true, secure: true, supportFetchAPI: true, corsEnabled: true}}]);
let exportsStarted = 0;
let window;
let timer;

app.whenReady().then(async () => {
  const ses = session.defaultSession;
  ses.setPermissionRequestHandler((_contents, _permission, callback) => callback(false));
  ses.setPermissionCheckHandler(() => false);
  ses.webRequest.onBeforeRequest({urls: ['<all_urls>']}, (details, callback) => {
    callback({cancel: policy.assetName(details.url) === null && !details.url.startsWith('blob:optic-alpha://app/')});
  });
  await ses.protocol.handle('optic-alpha', request => {
    const name = policy.assetName(request.url);
    if (!name) return new Response('Denied', {status: 403});
    const types = {'index.html': 'text/html; charset=utf-8', 'style.css': 'text/css; charset=utf-8', 'app.js': 'text/javascript; charset=utf-8', 'model.js': 'text/javascript; charset=utf-8'};
    return new Response(snapshot.assets.get(name), {headers: {'Content-Type': types[name], 'Content-Security-Policy': policy.CSP, 'Cache-Control': 'no-store'}});
  });
  ses.on('will-download', (event, item) => {
    if (!policy.exportAllowed(item.getURL(), item.getMimeType(), item.getTotalBytes(), exportsStarted)) {
      event.preventDefault(); return;
    }
    const folder = path.join(snapshot.root, 'exports');
    if (fs.existsSync(folder) && (!fs.lstatSync(folder).isDirectory() || fs.lstatSync(folder).isSymbolicLink())) {
      event.preventDefault(); return;
    }
    fs.mkdirSync(folder, {recursive: true});
    const target = path.join(folder, 'agenda-budget-' + crypto.randomUUID() + '.json');
    const fd = fs.openSync(target, 'wx'); fs.closeSync(fd);
    exportsStarted++;
    item.setSavePath(target);
    item.on('updated', () => { if (item.getReceivedBytes() > 65536) item.cancel(); });
    item.once('done', (_event, state) => {
      console.log('VIEWER_EXPORT', state);
      if (smoke && state === 'completed') {
        try { assert.equal(fs.readFileSync(target, 'utf8'), '{"public":true}'); console.log('ELECTRON_VIEWER_SMOKE_PASS'); app.quit(); }
        catch { console.error('viewer.export_probe_failed'); app.exit(1); }
      }
    });
  });
  window = new BrowserWindow({width: 1180, height: 820, minWidth: 700, minHeight: 500,
    title: 'Agenda & budget — test Optic', show: !smoke,
    webPreferences: {nodeIntegration: false, contextIsolation: true, sandbox: true,
      webSecurity: true, webviewTag: false, navigateOnDragDrop: false}});
  window.setMenuBarVisibility(false);
  window.webContents.setWindowOpenHandler(() => ({action: 'deny'}));
  window.webContents.on('will-navigate', event => event.preventDefault());
  window.webContents.on('will-attach-webview', event => event.preventDefault());
  window.webContents.on('will-prevent-unload', event => event.preventDefault());
  timer = setTimeout(() => { ses.flushStorageData(); app.exit(smoke ? 1 : 0); }, smoke ? 20000 : 3000000);
  await window.loadURL('optic-alpha://app/index.html');
  if (smoke) {
    if (repeat) assert.equal(await window.webContents.executeJavaScript("localStorage.getItem('public-probe')"), 'ok');
    assert.equal(await window.webContents.executeJavaScript('typeof require'), 'undefined');
    assert.equal(await window.webContents.executeJavaScript('typeof process'), 'undefined');
    assert.equal(await window.webContents.executeJavaScript("window.open('https://example.invalid') === null"), true);
    // A reachable owned listener distinguishes blocking from connection refusal.
    let connections = 0;
    const server = require('node:http').createServer((_request, response) => response.end('public'));
    server.on('connection', () => connections++);
    await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
    try {
      const url = 'http://127.0.0.1:' + server.address().port + '/';
      assert.equal(await (await fetch(url)).text(), 'public');
      assert.ok(connections > 0); connections = 0;
      assert.equal(await window.webContents.executeJavaScript('fetch(' + JSON.stringify(url) + ').then(()=>false,()=>true)'), true);
      assert.equal(connections, 0);
    } finally { server.closeAllConnections(); await new Promise(resolve => server.close(resolve)); }
    assert.equal(await window.webContents.executeJavaScript("fetch('file:///C:/Windows/win.ini').then(()=>false,()=>true)"), true);
    assert.equal(await window.webContents.executeJavaScript("fetch('optic-alpha://app/private.txt').then(r=>r.ok===false,()=>true)"), true);
    assert.equal(await window.webContents.executeJavaScript("localStorage.setItem('public-probe','ok');localStorage.getItem('public-probe')"), 'ok');
    await window.webContents.executeJavaScript("const blob=new Blob(['{\"public\":true}'],{type:'application/json'});const a=document.createElement('a');a.href=URL.createObjectURL(blob);a.download='public.json';a.click();");
  }
}).catch(() => { console.error('viewer.startup_failed'); app.exit(1); });
app.on('window-all-closed', () => app.quit());
app.on('will-quit', () => { clearTimeout(timer); if (app.isReady()) session.defaultSession.flushStorageData(); });
