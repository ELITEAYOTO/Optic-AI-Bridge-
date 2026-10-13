"""Local synthetic reusable-session contract. No effects, transport or persistence."""
from dataclasses import dataclass
import hashlib
import math
import re
import secrets
import time

from lifecycle_core import Identity

HANDLE = re.compile(r'ops_[0-9a-f]{96}\Z')
ADAPTERS = frozenset(('francestudent', 'synthetic-host'))


@dataclass(frozen=True)
class SyntheticSession:
    internal_id: str
    owner: Identity
    adapter: str
    deadline: float


class Resolver:
    def __init__(self, registry, clock=time.monotonic, capacity=32, ttl=120,
                 mint_budget=128, mint=lambda: secrets.token_hex(32)):
        if (type(capacity) is not int or not 1 <= capacity <= 32
                or type(ttl) not in (int, float) or not math.isfinite(ttl) or not 1 <= ttl <= 120
                or type(mint_budget) is not int or not 1 <= mint_budget <= 128):
            raise ValueError('invalid fixed limits')
        self.registry, self.clock, self.mint = registry, clock, mint
        self.capacity, self.ttl, self.mint_budget = capacity, ttl, mint_budget
        self.generation = secrets.token_hex(16)
        self.entries, self.issued = {}, set()
        self.last = clock()
        if not math.isfinite(self.last):
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
        return bool(state and state['active'] and state['principal'] == owner.principal
                    and state['epoch'] == owner.epoch)

    def _reap(self, now):
        self.entries = {key: value for key, value in self.entries.items()
                        if now < value.deadline and self._current(value.owner)}

    @staticmethod
    def _denied():
        # Avoid exposing whether handle exists, expired or belongs to another owner.
        return {'ok': False, 'code': 'probe.invalid_session'}

    def open(self, owner, adapter):
        with self.registry.lock:
            now = self._now()
            if now is None:
                return self._denied()
            self._reap(now)
            if not self._current(owner) or not isinstance(adapter, str) or adapter not in ADAPTERS:
                return self._denied()
            if len(self.entries) >= self.capacity or len(self.issued) >= self.mint_budget:
                return {'ok': False, 'code': 'probe.session_capacity'}
            suffix = self.mint()
            if not isinstance(suffix, str) or re.fullmatch(r'[0-9a-f]{64}', suffix) is None:
                return self._denied()
            handle = 'ops_' + self.generation + suffix
            digest = hashlib.sha256(handle.encode('ascii')).digest()
            if digest in self.issued:
                return self._denied()
            value = SyntheticSession('synthetic_' + secrets.token_hex(16), owner, adapter, now + self.ttl)
            self.issued.add(digest)
            self.entries[digest] = value
            return {'ok': True, 'application_handle': handle, 'expires_in_seconds': self.ttl,
                    'reusable': True, 'authority': 'none'}

    def resolve(self, owner, adapter, handle):
        with self.registry.lock:
            now = self._now()
            if now is None:
                return None
            self._reap(now)
            if (not self._current(owner) or not isinstance(adapter, str) or adapter not in ADAPTERS
                    or not isinstance(handle, str) or HANDLE.fullmatch(handle) is None):
                return None
            value = self.entries.get(hashlib.sha256(handle.encode('ascii')).digest())
            return value if value and value.owner == owner and value.adapter == adapter else None

    def close_local(self, owner, adapter, handle):
        # In-process operator lifecycle only. No remote close/cancel API.
        with self.registry.lock:
            now = self._now()
            if now is None:
                return self._denied()
            self._reap(now)
            if (not self._current(owner) or not isinstance(adapter, str) or adapter not in ADAPTERS
                    or not isinstance(handle, str) or HANDLE.fullmatch(handle) is None):
                return self._denied()
            key = hashlib.sha256(handle.encode('ascii')).digest()
            value = self.entries.get(key)
            if value is None or value.owner != owner or value.adapter != adapter:
                return self._denied()
            del self.entries[key]
            return {'ok': True, 'authority': 'none'}
