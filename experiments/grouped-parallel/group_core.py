"""Owner-bound fixed-delay grouped overlap. No project/session/effect authority."""
import asyncio
import hashlib
import hmac
import math
import secrets
import time

from lifecycle_core import Identity
from probe_core import HANDLE_RE


class Groups:
    def __init__(self, registry, clock=time.monotonic, sleeper=asyncio.sleep,
                 emit=lambda event: None, capacity=16, budget=64):
        if (type(capacity) is not int or not 1 <= capacity <= 16
                or type(budget) is not int or not 1 <= budget <= 64):
            raise ValueError('invalid fixed group limits')
        self.registry, self.clock, self.sleeper, self.emit = registry, clock, sleeper, emit
        self.capacity, self.budget = capacity, budget
        self.key = secrets.token_bytes(32)
        self.groups, self.issued = {}, set()
        self.active = 0
        self.started = self.last = clock()
        if not math.isfinite(self.started):
            raise ValueError('invalid monotonic clock')

    def _now(self):
        now = self.clock()
        if not math.isfinite(now) or now < self.last:
            return None
        self.last = now
        return now

    def _current(self, owner):
        if (not isinstance(owner, Identity) or not isinstance(owner.owner, str)
                or not isinstance(owner.principal, str) or type(owner.epoch) is not int):
            return False
        state = self.registry.states.get(owner.owner)
        return bool(state and state['active'] and state['epoch'] == owner.epoch
                    and state['principal'] == owner.principal)

    def _reap(self, now):
        self.groups = {key: group for key, group in self.groups.items()
                       if now < group['deadline'] and self._current(group['owner'])}

    @staticmethod
    def _digest(handle):
        return hashlib.sha256(handle.encode('ascii')).digest() if (
            isinstance(handle, str) and HANDLE_RE.fullmatch(handle)) else None

    @staticmethod
    def _denied():
        return {'ok': False, 'code': 'probe.invalid_group'}

    def _get(self, owner, handle, now):
        self._reap(now)
        group = self.groups.get(self._digest(handle))
        return group if self._current(owner) and group and group['owner'] == owner else None

    def _emit(self, record):
        try:
            self.emit(record)
        except Exception:
            # Diagnostic sink failure must not leak values or prevent cleanup.
            pass

    def create(self, owner):
        with self.registry.lock:
            now = self._now()
            if now is None or not self._current(owner):
                return self._denied()
            self._reap(now)
            if len(self.groups) >= self.capacity or len(self.issued) >= self.budget:
                return {'ok': False, 'code': 'probe.group_capacity'}
            handle = 'oph_' + hmac.new(self.key, b'group\0' + secrets.token_bytes(32), hashlib.sha256).hexdigest()
            digest = self._digest(handle)
            if digest in self.issued:
                return self._denied()
            group_id = secrets.token_hex(8)
            self.issued.add(digest)
            self.groups[digest] = {'owner': owner, 'deadline': now + 120,
                'group_id': group_id, 'records': [], 'active': set()}
            return {'ok': True, 'application_handle': handle, 'group_id': group_id,
                    'expires_in_seconds': 120, 'delay_ms': 3000, 'authority': 'none'}

    def _enter(self, owner, handle, sequence):
        with self.registry.lock:
            now = self._now()
            group = self._get(owner, handle, now) if now is not None else None
            if group is None:
                return None, self._denied()
            if len(group['records']) >= 8 or len(group['active']) >= 4 or self.active >= 8:
                return None, {'ok': False, 'code': 'probe.group_admission'}
            invocation = secrets.token_hex(8)
            if any(item['invocation_id'] == invocation for item in group['records']):
                return None, self._denied()
            record = {'invocation_id': invocation, 'request_sequence': sequence if type(sequence) is int and 1 <= sequence <= 512 else None,
                'entered_ms': round((now - self.started) * 1000, 3), 'finished_ms': None,
                'state': 'pending', 'active_on_entry': len(group['active']) + 1,
                'overlap_observed': bool(group['active'])}
            if group['active']:
                for item in group['records']:
                    if item['invocation_id'] in group['active']:
                        item['overlap_observed'] = True
            group['records'].append(record)
            group['active'].add(invocation)
            self.active += 1
            return (group, record, dict(record)), None

    def _finish(self, owner, group, record, state):
        with self.registry.lock:
            group['active'].remove(record['invocation_id'])
            self.active -= 1
            now = self._now()
            record['finished_ms'] = round((now - self.started) * 1000, 3) if now is not None else None
            record['state'] = state if now is not None else 'clock_invalid'
            valid = now is not None and now < group['deadline'] and self._current(owner)
            snapshot = dict(record)
            return valid, snapshot

    async def wait(self, owner, handle, sequence=None):
        entered, error = self._enter(owner, handle, sequence)
        if error:
            return error
        group, record, entry_snapshot = entered
        state = 'completed'
        try:
            self._emit({'phase': 'enter', 'group_id': group['group_id'], **entry_snapshot})
            await self.sleeper(3)
        except asyncio.CancelledError:
            state = 'cancelled'
            raise
        except Exception:
            state = 'failed'
        finally:
            valid, snapshot = self._finish(owner, group, record, state)
            self._emit({'phase': 'exit', 'group_id': group['group_id'], **snapshot})
        if not valid:
            return self._denied()
        if state != 'completed':
            return {'ok': False, 'code': 'probe.wait_failed'}
        return {'ok': True, 'group_id': group['group_id'], **snapshot, 'authority': 'none'}

    def report(self, owner, handle):
        with self.registry.lock:
            now = self._now()
            group = self._get(owner, handle, now) if now is not None else None
            if group is None:
                return self._denied()
            records = [dict(item) for item in group['records']]
            completed = [item for item in records if item['state'] == 'completed']
            overlap = any(max(a['entered_ms'], b['entered_ms']) < min(a['finished_ms'], b['finished_ms'])
                          for i, a in enumerate(completed) for b in completed[i + 1:])
            return {'ok': True, 'group_id': group['group_id'], 'records': records,
                    'active': len(group['active']), 'completed_pair_overlap': overlap,
                    'expires_in_seconds': math.ceil(group['deadline'] - now), 'authority': 'none'}
