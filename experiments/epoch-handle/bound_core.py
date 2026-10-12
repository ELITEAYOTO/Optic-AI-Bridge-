"""Principal/epoch-bound single-use dummy handoff. Zero authority."""
import hashlib
import secrets
import time

from lifecycle_core import Identity
from probe_core import HANDLE_RE


class EpochHandles:
    def __init__(self, registry, clock=time.monotonic,
                 mint=lambda: "oph_" + secrets.token_hex(32), capacity=32, ttl=120, mint_budget=128):
        self.registry, self.clock, self.mint = registry, clock, mint
        self.capacity, self.ttl = capacity, ttl
        self.entries = {}
        self.minted = set()
        self.mint_budget = mint_budget

    def _active(self, identity):
        # Caller holds registry.lock. The same lock serializes rotate/revoke and
        # the complete synthetic create/consume operation, including deletion.
        if not isinstance(identity, Identity):
            return False
        state = self.registry.states.get(identity.owner)
        return bool(state and state["active"] and state["epoch"] == identity.epoch
                    and state["principal"] == identity.principal)

    def _reap(self, now):
        self.entries = {key: item for key, item in self.entries.items()
                        if now < item[1] and self._active(item[0])}

    def create(self, identity):
        with self.registry.lock:
            now = self.clock()
            self._reap(now)
            if not self._active(identity):
                return {"ok": False, "code": "probe.authentication_inactive"}
            if len(self.entries) >= self.capacity:
                return {"ok": False, "code": "probe.capacity"}
            if len(self.minted) >= self.mint_budget:
                return {"ok": False, "code": "probe.mint_budget"}
            handle = self.mint()
            if not isinstance(handle, str) or not HANDLE_RE.fullmatch(handle):
                return {"ok": False, "code": "probe.mint_failed"}
            key = hashlib.sha256(handle.encode()).digest()
            if key in self.minted:
                return {"ok": False, "code": "probe.mint_failed"}
            self.minted.add(key)
            self.entries[key] = (identity, now + self.ttl)
            return {"ok": True, "application_handle": handle, "expires_in_seconds": self.ttl,
                    "single_use": True, "principal_id": identity.principal,
                    "auth_epoch": identity.epoch, "authority": "none"}

    def consume(self, identity, handle):
        with self.registry.lock:
            self._reap(self.clock())
            if not self._active(identity):
                return {"ok": False, "code": "probe.authentication_inactive"}
            if not isinstance(handle, str) or not HANDLE_RE.fullmatch(handle):
                return {"ok": False, "code": "probe.invalid_handle"}
            key = hashlib.sha256(handle.encode()).digest()
            item = self.entries.get(key)
            if item is None or item[0] != identity:
                return {"ok": False, "code": "probe.invalid_handle"}
            del self.entries[key]
            return {"ok": True, "passed_unchanged": True, "consumed": True,
                    "principal_id": identity.principal, "auth_epoch": identity.epoch,
                    "authority": "none"}
