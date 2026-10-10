from __future__ import annotations

from collections import OrderedDict
from dataclasses import dataclass
from hashlib import sha256
import hmac
import json
import re
import secrets
import threading
from typing import Any, Mapping

PROBE_VERSION = "h01-v1"
MAX_META_DEPTH = 4
MAX_META_ITEMS = 64
MAX_RUNS = 32
MAX_EVENTS_PER_RUN = 16
RUN_ID_RE = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._:-]{7,63}$")
META_KEY_RE = re.compile(r"^[A-Za-z0-9._:/-]{1,80}$")

# Fingerprints only need to correlate observations while one probe process is alive.
# A fresh key on every start prevents the probe output from becoming a durable
# cross-run tracking identifier or an easy offline dictionary target.
_FINGERPRINT_KEY = secrets.token_bytes(32)

_SENSITIVE_HEADER_FRAGMENTS = (
    "authorization",
    "cookie",
    "token",
    "secret",
    "api-key",
    "apikey",
    "credential",
)
_SAFE_HEADER_VALUES = {
    "mcp-protocol-version",
    "content-type",
    "accept",
}


def stable_fingerprint(value: Any) -> str:
    """Return a process-local one-way fingerprint for correlation, never authority."""
    try:
        encoded = json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True)
    except (TypeError, ValueError):
        encoded = repr(value)
    digest = hmac.new(_FINGERPRINT_KEY, encoded.encode("utf-8", "replace"), sha256)
    return digest.hexdigest()[:20]


def validate_run_id(run_id: str) -> str:
    if not RUN_ID_RE.fullmatch(run_id):
        raise ValueError(
            "run_id must be 8-64 characters using only letters, digits, '.', '_', ':', or '-'"
        )
    return run_id


def _is_sensitive_header(name: str) -> bool:
    lowered = name.lower()
    return any(fragment in lowered for fragment in _SENSITIVE_HEADER_FRAGMENTS)


def _safe_meta_key(raw_key: Any) -> str:
    key = str(raw_key)
    if META_KEY_RE.fullmatch(key):
        return key
    return f"key#{stable_fingerprint(key)}"


def sanitize_headers(headers: Mapping[str, str] | None) -> dict[str, Any]:
    if not headers:
        return {"present": False, "names": [], "safe_values": {}, "fingerprints": {}}

    names: list[str] = []
    safe_values: dict[str, str] = {}
    fingerprints: dict[str, str] = {}

    for raw_name, raw_value in headers.items():
        name = str(raw_name).lower().strip()
        if not name or _is_sensitive_header(name):
            continue
        names.append(name)
        value = str(raw_value)
        if name in _SAFE_HEADER_VALUES:
            safe_values[name] = value[:256]
        else:
            fingerprints[name] = stable_fingerprint(value)

    names = sorted(set(names))[:MAX_META_ITEMS]
    safe_values = {k: safe_values[k] for k in sorted(safe_values) if k in names}
    fingerprints = {k: fingerprints[k] for k in sorted(fingerprints) if k in names}
    return {
        "present": True,
        "names": names,
        "safe_values": safe_values,
        "fingerprints": fingerprints,
    }


def _summarize_meta_value(value: Any, depth: int) -> Any:
    if depth >= MAX_META_DEPTH:
        return {"type": type(value).__name__, "fingerprint": stable_fingerprint(value)}

    if value is None or isinstance(value, bool):
        return value

    if isinstance(value, (int, float)):
        return {"type": type(value).__name__, "fingerprint": stable_fingerprint(value)}

    if isinstance(value, str):
        return {"type": "string", "fingerprint": stable_fingerprint(value)}

    if isinstance(value, Mapping):
        items = list(value.items())[:MAX_META_ITEMS]
        return {
            _safe_meta_key(key): _summarize_meta_value(child, depth + 1)
            for key, child in sorted(items, key=lambda item: str(item[0]))
        }

    if isinstance(value, (list, tuple)):
        return {
            "type": "array",
            "length": len(value),
            "fingerprint": stable_fingerprint(value),
        }

    return {"type": type(value).__name__, "fingerprint": stable_fingerprint(value)}


def sanitize_meta(meta: Mapping[str, Any] | None) -> dict[str, Any]:
    if not meta:
        return {"present": False, "keys": [], "summary": {}, "fingerprints": {}}

    items = list(meta.items())[:MAX_META_ITEMS]
    normalized = [(_safe_meta_key(key), value) for key, value in items]
    summary = {
        key: _summarize_meta_value(value, 0)
        for key, value in sorted(normalized, key=lambda item: item[0])
    }
    fingerprints = {key: stable_fingerprint(value) for key, value in normalized}
    return {
        "present": True,
        "keys": sorted(summary),
        "summary": summary,
        "fingerprints": {key: fingerprints[key] for key in sorted(fingerprints)},
    }


def build_observation(
    *,
    request_id: Any,
    session_id: str | None,
    headers: Mapping[str, str] | None,
    meta: Mapping[str, Any] | None,
) -> dict[str, Any]:
    header_summary = sanitize_headers(headers)
    meta_summary = sanitize_meta(meta)
    return {
        "probe_version": PROBE_VERSION,
        "request_id_fingerprint": stable_fingerprint(str(request_id)),
        "transport_session": {
            "present": session_id is not None,
            "fingerprint": stable_fingerprint(session_id) if session_id is not None else None,
        },
        "headers": header_summary,
        "meta": meta_summary,
    }


def _compare_maps(first: Mapping[str, str], current: Mapping[str, str]) -> dict[str, bool]:
    shared = sorted(set(first).intersection(current))
    return {key: first[key] == current[key] for key in shared}


@dataclass(frozen=True)
class CorrelationResult:
    call_index: int
    same_transport_session_as_first: bool | None
    matching_header_fingerprints: dict[str, bool]
    matching_meta_fingerprints: dict[str, bool]


class CorrelationStore:
    """Small bounded in-memory store used only by the harmless H-01 probe."""

    def __init__(self, max_runs: int = MAX_RUNS, max_events_per_run: int = MAX_EVENTS_PER_RUN):
        self._max_runs = max_runs
        self._max_events_per_run = max_events_per_run
        self._runs: OrderedDict[str, list[dict[str, Any]]] = OrderedDict()
        self._lock = threading.Lock()

    def record(self, run_id: str, observation: dict[str, Any]) -> CorrelationResult:
        validate_run_id(run_id)
        with self._lock:
            events = self._runs.pop(run_id, [])
            if len(events) >= self._max_events_per_run:
                # Keep the original baseline stable. Silently dropping it would make a later
                # call appear to compare with "the first request" when it no longer does.
                self._runs[run_id] = events
                raise ValueError(
                    f"run_id reached the {self._max_events_per_run}-event limit; reset it before reuse"
                )

            events.append(observation)
            self._runs[run_id] = events
            while len(self._runs) > self._max_runs:
                self._runs.popitem(last=False)

            first = events[0]
            current = events[-1]
            first_session = first["transport_session"]["fingerprint"]
            current_session = current["transport_session"]["fingerprint"]
            if first_session is None or current_session is None:
                same_session: bool | None = None
            else:
                same_session = first_session == current_session

            return CorrelationResult(
                call_index=len(events),
                same_transport_session_as_first=same_session,
                matching_header_fingerprints=_compare_maps(
                    first["headers"]["fingerprints"], current["headers"]["fingerprints"]
                ),
                matching_meta_fingerprints=_compare_maps(
                    first["meta"]["fingerprints"], current["meta"]["fingerprints"]
                ),
            )

    def reset(self, run_id: str) -> bool:
        validate_run_id(run_id)
        with self._lock:
            return self._runs.pop(run_id, None) is not None
