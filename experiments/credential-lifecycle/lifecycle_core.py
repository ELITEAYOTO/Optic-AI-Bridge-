"""Finite dummy credential registry. No project authority, persistence or network."""
from dataclasses import dataclass
import hmac
import secrets
import threading

TOKENS = {"A": (b"optic-probe-dummy-owner-a", b"optic-probe-dummy-owner-a-v2"),
          "B": (b"optic-probe-dummy-owner-b", b"optic-probe-dummy-owner-b-v2")}


@dataclass(frozen=True)
class Identity:
    owner: str
    principal: str
    epoch: int


class Registry:
    def __init__(self):
        self.lock = threading.Lock()
        self.states = {owner: {"principal": secrets.token_hex(8), "epoch": 1,
                              "version": 0, "active": True} for owner in TOKENS}

    def authenticate(self, value):
        # Exact dummy comparison only; real credentials are never supported.
        with self.lock:
            found = None
            for owner, values in TOKENS.items():
                state = self.states[owner]
                matches = hmac.compare_digest(value, b"Bearer " + values[state["version"]])
                if matches and state["active"]:
                    found = Identity(owner, state["principal"], state["epoch"])
            return found

    def current(self, identity):
        with self.lock:
            if not isinstance(identity, Identity) or identity.owner not in self.states:
                return False
            state = self.states[identity.owner]
            return state["active"] and state["epoch"] == identity.epoch and state["principal"] == identity.principal

    def control(self, operation, owner):
        # Only fixed local commands can change state. One rotation per owner:
        # never cycle back to a formerly accepted dummy credential.
        with self.lock:
            if operation not in ("rotate", "revoke") or owner not in self.states:
                return {"ok": False, "code": "probe.invalid_local_command"}
            state = self.states[owner]
            if operation == "rotate":
                if state["version"] == 1:
                    return {"ok": False, "code": "probe.rotation_limit"}
                state["version"] = 1
                state["active"] = True
            else:
                state["active"] = False
            state["epoch"] += 1
            return {"ok": True, "operation": operation, "owner": owner,
                    "epoch": state["epoch"], "active": state["active"]}
