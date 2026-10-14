'use strict';
const test = require('node:test'), assert = require('node:assert/strict');
const fs = require('node:fs'), os = require('node:os'), path = require('node:path'), crypto = require('node:crypto');
const policy = require('./viewer_policy.cjs');

test('exact assets only; no query, host, traversal, credentials or tests', () => {
  assert.equal(policy.assetName('optic-alpha://app/index.html'), 'index.html');
  for (const url of ['file:///C:/Windows/win.ini', 'https://app/index.html', 'optic-alpha://evil/index.html',
    'optic-alpha://app/tests.js', 'optic-alpha://app/%2e%2e/private.txt', 'optic-alpha://user@app/app.js',
    'optic-alpha://app/app.js?x=1', 'optic-alpha://app/app.js#x']) assert.equal(policy.assetName(url), null);
});

test('exports have a finite bound and fixed blob origin and MIME', () => {
  const url = 'blob:optic-alpha://app/abcd-1234';
  assert.equal(policy.exportAllowed(url, 'application/json', 24, 0), true);
  for (const bytes of [-1, 0, 65537, Infinity, 1.5]) assert.equal(policy.exportAllowed(url, 'application/json', bytes, 0), false);
  assert.equal(policy.exportAllowed(url, 'application/json', 24, 8), false);
  assert.equal(policy.exportAllowed('blob:https://evil/abcd', 'application/json', 24, 0), false);
  assert.equal(policy.exportAllowed(url, 'text/html', 24, 0), false);
});

test('unknown Windows volume is explicit; known volume/inode/size conflicts still denied', () => {
  const info = {dev: 0n, ino: 9007199254740995n, size: 6n};
  const opened = {...info, dev: 742408122n, isFile: () => true};
  assert.equal(policy.matchingOpenedFile(info, opened, 'win32'), true);
  assert.equal(policy.matchingOpenedFile(info, opened, 'linux'), false);
  assert.equal(policy.matchingOpenedFile({...info, dev: 42n}, opened, 'win32'), false);
  assert.equal(policy.matchingOpenedFile(info, {...opened, ino: info.ino + 1n}, 'win32'), false);
  assert.equal(policy.matchingOpenedFile(info, {...opened, size: 7n}, 'win32'), false);
  assert.equal(policy.matchingOpenedFile(info, {...opened, isFile: () => false}, 'win32'), false);
});

test('snapshot is generated-root-only; content changes refused; held bytes are copies', () => {
  const parent = fs.mkdtempSync(path.join(os.tmpdir(), 'optic-viewer-policy-'));
  const root = path.join(parent, 'alpha-public-test'), src = path.join(root, 'project', 'src');
  const created = [];
  try {
    fs.mkdirSync(src, {recursive: true});
    const versions = {};
    for (const name of policy.FILES) {
      const file = path.join(src, name); fs.writeFileSync(file, 'public'); created.push(file);
      versions[name] = crypto.createHash('sha256').update('public').digest('hex');
    }
    const marker = path.join(root, 'viewer-ready.json'); created.push(marker);
    fs.writeFileSync(marker, JSON.stringify({version: 1, files: versions}));
    let snapshot;
    try { snapshot = policy.loadSnapshot(root, parent); }
    catch (error) {
      const fd = fs.openSync(marker, 'r');
      try {
        const a = fs.lstatSync(marker, {bigint: true}), b = fs.fstatSync(fd, {bigint: true});
        console.error('PUBLIC_SNAPSHOT_STAT', ['dev','ino','size'].map(key => [key, String(a[key]), String(b[key])]));
      } finally { fs.closeSync(fd); }
      throw error;
    }
    assert.equal(snapshot.assets.has('tests.js'), false);
    assert.throws(() => policy.loadSnapshot(root, src));
    fs.writeFileSync(path.join(src, 'model.js'), 'changed');
    assert.throws(() => policy.loadSnapshot(root, parent), /source_changed/);
    assert.equal(snapshot.assets.get('model.js').toString(), 'public');
  } finally {
    for (const file of created) if (fs.existsSync(file)) fs.unlinkSync(file);
    for (const dir of [src, path.dirname(src), root, parent]) fs.rmdirSync(dir);
  }
});
