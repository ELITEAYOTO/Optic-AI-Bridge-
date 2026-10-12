"""Authority-free dummy-header and opaque handoff contract. No runtime imports."""
from __future__ import annotations

import hashlib
import hmac
import re
import secrets
import threading
import time

DUMMY_TOKENS = {"A": b"optic-probe-dummy-owner-a", "B": b"optic-probe-dummy-owner-b"}
DUMMY_HEADERS = {b"x-optic-probe": b"optic-probe-dummy-header", b"x-optic-probe-secret": b"optic-probe-dummy-secret"}
HANDLE_RE = re.compile(r"^oph_[0-9a-f]{64}$")
MAX_HANDLES = 32
HANDLE_TTL = 120


def observe_headers(headers):
    """Only constant names and booleans leave this function; never fingerprint secrets."""
    auth = [v for k, v in headers if k.lower() == b"authorization"]
    value = auth[0] if len(auth) == 1 else b""
    bearer = value.startswith(b"Bearer ")
    owner = next((k for k, v in DUMMY_TOKENS.items()
                  if len(auth) == 1 and hmac.compare_digest(value, b"Bearer " + v)), None)
    custom = {}
    for name, expected in DUMMY_HEADERS.items():
        values = [v for k, v in headers if k.lower() == name]
        custom[name.decode()] = {"present": bool(values), "single": len(values) == 1,
                                "matches_dummy": len(values) == 1 and hmac.compare_digest(values[0], expected)}
    return owner, {"authorization_present": bool(auth), "authorization_single": len(auth) == 1,
                   "scheme": "Bearer" if bearer else "absent_or_other",
                   "matches_dummy": owner is not None, "custom": custom}


class Handles:
    """In-memory handoff tokens, not SessionHandles or capabilities.

    Only a digest is retained. Owner mismatch does not consume another owner's
    token. Success atomically deletes it. No tombstones or unbounded history.
    """
    def __init__(self, clock=time.monotonic, mint=lambda: "oph_" + secrets.token_hex(32),
                 capacity=MAX_HANDLES, ttl=HANDLE_TTL):
        self.clock, self.mint, self.capacity, self.ttl = clock, mint, capacity, ttl
        self.entries = {}
        self.lock = threading.Lock()

    def _reap(self, now):
        self.entries = {k: v for k, v in self.entries.items() if now < v[1]}

    def create(self, owner):
        with self.lock:
            now = self.clock()
            self._reap(now)
            if owner not in DUMMY_TOKENS:
                return {"ok": False, "code": "probe.owner_required"}
            if len(self.entries) >= self.capacity:
                return {"ok": False, "code": "probe.capacity"}
            # Collisions fail closed rather than replacing an existing owner.
            handle = self.mint()
            if not HANDLE_RE.fullmatch(handle):
                return {"ok": False, "code": "probe.mint_failed"}
            key = hashlib.sha256(handle.encode()).digest()
            if key in self.entries:
                return {"ok": False, "code": "probe.mint_failed"}
            self.entries[key] = (owner, now + self.ttl)
            return {"ok": True, "application_handle": handle, "expires_in_seconds": self.ttl,
                    "single_use": True, "authority": "none"}

    def consume(self, owner, handle):
        with self.lock:
            self._reap(self.clock())
            if not isinstance(handle, str) or not HANDLE_RE.fullmatch(handle):
                return {"ok": False, "code": "probe.invalid_handle"}
            key = hashlib.sha256(handle.encode()).digest()
            item = self.entries.get(key)
            if owner not in DUMMY_TOKENS or item is None or item[0] != owner:
                return {"ok": False, "code": "probe.invalid_handle"}
            del self.entries[key]
            return {"ok": True, "passed_unchanged": True, "consumed": True, "authority": "none"}
