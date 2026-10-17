"""Operator-only snapshot after confirmed core session stop; outside project scope."""
import hashlib
import json
from pathlib import Path

FILES = ('index.html', 'style.css', 'app.js', 'model.js', 'tests.js')


def write_viewer_ready(run):
    run = Path(run)
    source = run / 'project' / 'src'
    for directory in (run, run / 'project', source):
        if directory.is_symlink() or not directory.is_dir():
            raise ValueError('viewer.invalid_directory')
    versions = {}
    for name in FILES:
        path = source / name
        if path.is_symlink() or not path.is_file():
            raise ValueError('viewer.missing_source')
        with path.open('rb') as stream:
            raw = stream.read(8193)
        if not 1 <= len(raw) <= 8192:
            raise ValueError('viewer.source_bound')
        raw.decode('utf-8', errors='strict')
        versions[name] = hashlib.sha256(raw).hexdigest()
    with (run / 'viewer-ready.json').open('x', encoding='utf-8') as stream:
        json.dump({'version': 1, 'files': versions}, stream, sort_keys=True)
    return {'viewer_ready': True}
