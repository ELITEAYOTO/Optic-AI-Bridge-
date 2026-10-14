'use strict';
const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const FILES = Object.freeze(['index.html', 'style.css', 'app.js', 'model.js', 'tests.js']);
const ASSETS = Object.freeze(FILES.filter(name => name !== 'tests.js'));
const CSP = "default-src 'none'; script-src 'self'; style-src 'self'; img-src 'self' data:; connect-src 'none'; object-src 'none'; base-uri 'none'; frame-src 'none'; form-action 'none'";

function directory(value) {
  const info = fs.lstatSync(value);
  if (!info.isDirectory() || info.isSymbolicLink()) throw new Error('viewer.invalid_directory');
  return fs.realpathSync(value);
}

function readBounded(file, limit) {
  const info = fs.lstatSync(file);
  if (!info.isFile() || info.isSymbolicLink() || info.size < 1 || info.size > limit) throw new Error('viewer.invalid_file');
  const fd = fs.openSync(file, 'r');
  try {
    const opened = fs.fstatSync(fd);
    if (!opened.isFile() || opened.dev !== info.dev || opened.ino !== info.ino || opened.size !== info.size) throw new Error('viewer.changed_file');
    const bytes = Buffer.alloc(limit + 1);
    const length = fs.readSync(fd, bytes, 0, bytes.length, 0);
    if (length !== info.size || length > limit) throw new Error('viewer.changed_file');
    return bytes.subarray(0, length);
  } finally { fs.closeSync(fd); }
}

function loadSnapshot(run, output) {
  const root = directory(run), parent = directory(output);
  if (path.dirname(root) !== parent || !/^alpha-[a-z0-9_-]{1,64}$/i.test(path.basename(root))) throw new Error('viewer.outside_generated_output');
  const project = directory(path.join(root, 'project'));
  const source = directory(path.join(project, 'src'));
  if (path.dirname(project) !== root || path.dirname(source) !== project) throw new Error('viewer.invalid_source');
  const ready = JSON.parse(readBounded(path.join(root, 'viewer-ready.json'), 2048).toString('utf8'));
  if (ready.version !== 1 || Object.keys(ready).sort().join(',') !== 'files,version'
      || Object.keys(ready.files).sort().join(',') !== [...FILES].sort().join(',')) throw new Error('viewer.invalid_snapshot');
  const assets = new Map();
  for (const name of FILES) {
    const bytes = readBounded(path.join(source, name), 8192);
    const digest = crypto.createHash('sha256').update(bytes).digest('hex');
    if (digest !== ready.files[name]) throw new Error('viewer.source_changed');
    if (ASSETS.includes(name)) assets.set(name, Buffer.from(bytes));
  }
  return {root, assets};
}

function assetName(url) {
  try {
    const value = new URL(url);
    if (value.protocol !== 'optic-alpha:' || value.hostname !== 'app' || value.port || value.username || value.password || value.search || value.hash) return null;
    const name = value.pathname.slice(1);
    return ASSETS.includes(name) ? name : null;
  } catch { return null; }
}

function exportAllowed(url, mime, bytes, count) {
  return typeof url === 'string' && /^blob:optic-alpha:\/\/app\/[a-z0-9-]+$/i.test(url)
    && mime === 'application/json' && Number.isSafeInteger(bytes) && bytes >= 2 && bytes <= 65536
    && Number.isSafeInteger(count) && count >= 0 && count < 8;
}

module.exports = Object.freeze({FILES, ASSETS, CSP, loadSnapshot, assetName, exportAllowed});
