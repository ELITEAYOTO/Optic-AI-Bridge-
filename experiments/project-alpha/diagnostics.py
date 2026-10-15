"""Bounded model-readable tool outcomes, never raw requests or FS internals."""
from collections import deque
from datetime import datetime, timezone
import math
import re
from lifecycle_core import Identity

DIAGNOSTIC_TOOL = 'optic_diagnostics'
MAX_EVENTS = 32
PAGE_EVENTS = 8
MAX_COUNTER = (1 << 63) - 1
CODE = re.compile(r'(?:optic|project|execution|link)\.[a-z_]{1,64}\Z', re.ASCII)
TOOL = re.compile(r'optic_[a-z_]{1,48}\Z', re.ASCII)


class ToolDiagnostics:
    def __init__(self):
        self.binding = None
        self.events = deque(maxlen=MAX_EVENTS)
        self.recorded = 0

    @staticmethod
    def _current_locked(gate, execution, identity):
        # Caller owns the registry lock; never call Registry.current recursively.
        if not isinstance(identity, Identity) or gate.stopped or execution is not None and execution.stopped:
            return False
        binding = gate.registry.states.get(identity.owner)
        return (binding is not None and binding['active']
                and identity.owner == gate.owner and identity.principal == gate.principal
                and identity.principal == binding['principal'] and identity.epoch == binding['epoch'])

    def record_current(self, gate, execution, identity, *arguments):
        with gate.registry.lock:
            if self._current_locked(gate, execution, identity):
                self.record(identity, *arguments)

    def snapshot_current(self, gate, execution, identity, pending, retained_jobs):
        with gate.registry.lock:
            if not self._current_locked(gate, execution, identity):
                return None
            return self.snapshot(identity, pending, retained_jobs)

    def _bind(self, identity):
        binding = (identity.principal, identity.epoch)
        if self.binding != binding:
            self.binding = binding
            self.events.clear()
            self.recorded = 0

    def record(self, identity, name, payload, sequence, admitted, finished, received_utc=None):
        self._bind(identity)
        if name == DIAGNOSTIC_TOOL or not TOOL.fullmatch(name):
            return
        code = payload.get('code') if payload.get('ok') is False else None
        if code is not None and (type(code) is not str or not CODE.fullmatch(code)):
            code = 'project.unclassified_error'
        valid_time = all(type(value) in (int, float) and math.isfinite(value) for value in (admitted, finished))
        elapsed = min(60000, round(max(0, finished - admitted) * 1000, 3)) if valid_time else None
        received = None
        if type(received_utc) is str and len(received_utc) <= 40:
            try:
                parsed = datetime.fromisoformat(received_utc)
                if parsed.tzinfo is not None:
                    received = parsed.astimezone(timezone.utc).isoformat(timespec='milliseconds')
            except ValueError:
                pass
        self.recorded = min(MAX_COUNTER, self.recorded + 1)
        self.events.append({'sequence': sequence if type(sequence) is int and 0 <= sequence <= MAX_COUNTER else None,
            'tool': name, 'ok': payload.get('ok') is True, 'code': code,
            'outcome_uncertain': code in ('project.outcome_unknown', 'execution.outcome_unknown'),
            'received_utc': received,
            'finished_utc': datetime.now(timezone.utc).isoformat(timespec='milliseconds'),
            'request_elapsed_ms': elapsed, 'duration_capped': bool(valid_time and finished - admitted > 60)})

    def snapshot(self, identity, pending, retained_jobs):
        self._bind(identity)
        return {'events': list(self.events)[-PAGE_EVENTS:],
                'recorded': self.recorded, 'evicted': max(0, self.recorded - MAX_EVENTS),
                'older_in_memory': max(0, len(self.events) - PAGE_EVENTS),
                'pending_forwarding': bool(pending), 'retained_jobs': retained_jobs,
                'scope': 'current_bridge_process_and_credential_epoch',
                'persisted': False, 'fs_backend_visibility': False}
