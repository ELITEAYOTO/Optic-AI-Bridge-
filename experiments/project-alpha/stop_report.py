"""Bounded operator-only stop evidence, independent of console delivery."""
import json
import os
from pathlib import Path
import tempfile

from project_resume import ordinary


def write_stop_report(root, outcome, reason, diagnostics_drained, snapshot):
    root = Path(root)
    ordinary(root, directory=True)
    if reason not in ('operator', 'expiry', 'service_exit'):
        raise ValueError('project.stop_reason')
    report = {'version': 1, 'reason': reason,
              'revocation_confirmed': outcome.get('ok') is True
                  and outcome.get('session_revoked') is True,
              'work_drained': outcome.get('drained') is True,
              'diagnostics_drained': diagnostics_drained is True,
              'diagnostics_failed': snapshot['failed'] is True,
              **{key: snapshot[key] for key in ('accepted', 'delivered', 'dropped', 'queued')}}
    for key in ('accepted', 'delivered', 'dropped', 'queued'):
        if type(report[key]) is not int or not 0 <= report[key] <= 4096:
            raise ValueError('project.stop_counters')
    raw = json.dumps(report, sort_keys=True).encode('ascii')
    if len(raw) > 1024:
        raise ValueError('project.stop_report_bound')
    # One retained report, one owned transient file, no source/credential data.
    candidate = None
    try:
        with tempfile.NamedTemporaryFile(prefix='.stop-', suffix='.tmp', dir=root, delete=False) as stream:
            candidate = Path(stream.name)
            stream.write(raw)
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(candidate, root / 'last-stop.json')
    finally:
        if candidate is not None and candidate.exists():
            candidate.unlink()
    return report
