"""Ephemeral idempotent bootstrap candidate. Fixed state; no authority/effects."""
from dataclasses import dataclass
import hashlib
import hmac
import math
import secrets
import time

from resolver_core import ADAPTERS, SyntheticSession
from probe_core import HANDLE_RE
from lifecycle_core import Identity


@dataclass
class Ticket:
    owner: Identity
    adapter: str
    deadline: float
    session_key: bytes | None = None
    closed: bool = False


class BootstrapResolver:
    def __init__(self, registry, clock=time.monotonic, capacity=32, budget=128,
                 ttl=120, mint=lambda: secrets.token_bytes(32)):
        if (type(capacity) is not int or not 1 <= capacity <= 32
                or type(budget) is not int or not 1 <= budget <= 128
                or type(ttl) is not int or not 1 <= ttl <= 120):
            raise ValueError('invalid fixed limits')
        self.registry, self.clock, self.mint = registry, clock, mint
        self.capacity, self.budget, self.ttl = capacity, budget, ttl
        self.key = secrets.token_bytes(32)
        self.tickets, self.sessions = {}, {}
        self.last = clock()
        if not math.isfinite(self.last):
            raise ValueError('invalid clock')

    def _now(self):
        value = self.clock()
        if not math.isfinite(value) or value < self.last:
            return None
        self.last = value
        return value

    def _current(self, owner):
        if (not isinstance(owner, Identity) or not isinstance(owner.owner, str)
                or not isinstance(owner.principal, str) or type(owner.epoch) is not int):
            return False
        state = self.registry.states.get(owner.owner)
        return bool(state and state['active'] and state['epoch'] == owner.epoch
                    and state['principal'] == owner.principal)

    def _eligible(self, owner, adapter):
        return self._current(owner) and isinstance(adapter, str) and adapter in ADAPTERS

    def _reap(self, now):
        for ticket in self.tickets.values():
            if ticket.closed or now >= ticket.deadline or not self._current(ticket.owner):
                ticket.closed = True
                if ticket.session_key is not None:
                    self.sessions.pop(ticket.session_key, None)

    @staticmethod
    def _digest(handle):
        if not isinstance(handle, str) or HANDLE_RE.fullmatch(handle) is None:
            return None
        return hashlib.sha256(handle.encode('ascii')).digest()

    @staticmethod
    def _denied():
        return {'ok': False, 'code': 'probe.invalid_session'}

    def bootstrap(self, owner, adapter):
        with self.registry.lock:
            now = self._now()
            if now is None:
                return self._denied()
            self._reap(now)
            if not self._eligible(owner, adapter):
                return self._denied()
            if len(self.tickets) >= self.budget:
                return {'ok': False, 'code': 'probe.bootstrap_budget'}
            random = self.mint()
            if not isinstance(random, bytes) or len(random) != 32:
                return self._denied()
            # Domain-separated process-key derivation: no old ticket reissue
            # even if test mint repeats across fresh process generations.
            handle = 'oph_' + hmac.new(self.key, b'ticket\0' + random, hashlib.sha256).hexdigest()
            digest = self._digest(handle)
            if digest in self.tickets or digest in self.sessions:
                return self._denied()
            self.tickets[digest] = Ticket(owner, adapter, now + self.ttl)
            return {'ok': True, 'application_handle': handle, 'kind': 'bootstrap_ticket',
                    'expires_in_seconds': self.ttl, 'authority': 'none'}

    def open(self, owner, adapter, bootstrap_ticket):
        with self.registry.lock:
            now = self._now()
            if now is None:
                return self._denied()
            self._reap(now)
            ticket_key = self._digest(bootstrap_ticket)
            ticket = self.tickets.get(ticket_key)
            if (not self._eligible(owner, adapter) or ticket is None or ticket.closed
                    or ticket.owner != owner or ticket.adapter != adapter):
                return self._denied()
            handle = 'oph_' + hmac.new(self.key, b'session\0' + ticket_key, hashlib.sha256).hexdigest()
            session_key = self._digest(handle)
            reused = ticket.session_key is not None
            if not reused:
                if len(self.sessions) >= self.capacity:
                    return {'ok': False, 'code': 'probe.session_capacity'}
                if session_key in self.sessions or session_key in self.tickets:
                    return self._denied()
                self.sessions[session_key] = SyntheticSession('synthetic_' + secrets.token_hex(16),
                    owner, adapter, ticket.deadline)
                ticket.session_key = session_key
            session = self.sessions.get(session_key)
            if session is None or ticket.session_key != session_key:
                return self._denied()
            return {'ok': True, 'application_handle': handle, 'kind': 'session_reference',
                    'synthetic_session_id': session.internal_id, 'reused': reused,
                    'expires_in_seconds': math.ceil(ticket.deadline - now), 'authority': 'none'}

    def probe(self, owner, adapter, handle):
        with self.registry.lock:
            now = self._now()
            if now is None:
                return self._denied()
            self._reap(now)
            session = self.sessions.get(self._digest(handle))
            if (not self._eligible(owner, adapter) or session is None
                    or session.owner != owner or session.adapter != adapter):
                return self._denied()
            return {'ok': True, 'synthetic_session_id': session.internal_id,
                    'principal_id': owner.principal, 'auth_epoch': owner.epoch,
                    'expires_in_seconds': math.ceil(session.deadline - now), 'authority': 'none'}

    def close_owner_local(self, owner):
        with self.registry.lock:
            if owner not in ('A', 'B'):
                return self._denied()
            for ticket in self.tickets.values():
                if ticket.owner.owner == owner:
                    ticket.closed = True
                    if ticket.session_key is not None:
                        self.sessions.pop(ticket.session_key, None)
            return {'ok': True, 'operation': 'close', 'owner': owner, 'authority': 'none'}
