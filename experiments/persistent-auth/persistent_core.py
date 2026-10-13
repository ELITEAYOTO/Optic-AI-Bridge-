"""Process-restart fixture for PUBLIC DUMMY auth metadata, not secret storage."""
import copy
import json
import os
from pathlib import Path
import re
import stat

from lifecycle_core import Registry

MAX_STATE = 4096
HEX = re.compile(r"^[0-9a-f]{16}$")


class FixtureError(Exception):
    def __init__(self, code):
        super().__init__(code)
        self.code = code


def validate(data):
    if not isinstance(data, dict) or set(data) != {"schema", "owners"} or type(data["schema"]) is not int or data["schema"] != 1:
        raise FixtureError("fixture.invalid_state")
    owners = data["owners"]
    if not isinstance(owners, dict) or set(owners) != {"A", "B"}:
        raise FixtureError("fixture.invalid_state")
    for value in owners.values():
        if (not isinstance(value, dict) or set(value) != {"principal", "epoch", "version", "active"}
            or not isinstance(value["principal"], str) or not HEX.fullmatch(value["principal"])
            or type(value["epoch"]) is not int or not 1 <= value["epoch"] <= 65535
            or type(value["version"]) is not int or value["version"] not in (0, 1)
            or type(value["active"]) is not bool):
            raise FixtureError("fixture.invalid_state")
        if (value["version"] == 1 or not value["active"]) and value["epoch"] < 2:
            raise FixtureError("fixture.invalid_state")
    if owners["A"]["principal"] == owners["B"]["principal"]:
        raise FixtureError("fixture.invalid_state")
    return owners


def unique_object(pairs):
    value = {}
    for key, item in pairs:
        if key in value:
            raise FixtureError("fixture.invalid_state")
        value[key] = item
    return value


def ordinary(path, directory=False, missing=False):
    try:
        info = path.lstat()
    except FileNotFoundError:
        if missing:
            return
        raise FixtureError("fixture.missing_state") from None
    if (stat.S_ISLNK(info.st_mode) or getattr(info, "st_file_attributes", 0) & getattr(stat, "FILE_ATTRIBUTE_REPARSE_POINT", 1024)
        or (directory and not stat.S_ISDIR(info.st_mode)) or (not directory and not stat.S_ISREG(info.st_mode))):
        raise FixtureError("fixture.invalid_path")


class Store:
    """Exclusive cooperating-writer OS lock, bounded file, replace-before-ACK."""
    def __init__(self, directory):
        self.directory = Path(directory)
        ordinary(self.directory, directory=True)
        self.path = self.directory / "state.json"
        self.pending = self.directory / "state.pending"
        lock = self.directory / "writer.lock"
        ordinary(lock, missing=True)
        self.fd = os.open(lock, os.O_RDWR | os.O_CREAT | getattr(os, "O_NOFOLLOW", 0), 0o600)
        try:
            if os.fstat(self.fd).st_size == 0:
                os.write(self.fd, b"0")
            os.lseek(self.fd, 0, os.SEEK_SET)
            if os.name == "nt":
                import msvcrt
                msvcrt.locking(self.fd, msvcrt.LK_NBLCK, 1)
            else:
                import fcntl
                fcntl.flock(self.fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except OSError:
            os.close(self.fd)
            self.fd = None
            raise FixtureError("fixture.writer_busy") from None

    @classmethod
    def initialize(cls, directory):
        # Existing directory can NEVER be silently reinitialized, even if its
        # state file was removed. Explicit fresh fixture path only.
        directory = Path(directory)
        try:
            directory.mkdir()
        except FileExistsError:
            raise FixtureError("fixture.already_exists") from None
        store = cls(directory)
        try:
            store.write(Registry().states, initialize=True)
        finally:
            store.close()

    def read(self):
        ordinary(self.directory, directory=True)
        ordinary(self.path)
        try:
            with self.path.open("rb") as file:
                raw = file.read(MAX_STATE + 1)
            if len(raw) > MAX_STATE:
                raise FixtureError("fixture.invalid_state")
            return copy.deepcopy(validate(json.loads(raw, object_pairs_hook=unique_object)))
        except (ValueError, UnicodeError, RecursionError):
            raise FixtureError("fixture.invalid_state") from None

    def write(self, owners, initialize=False):
        data = {"schema": 1, "owners": owners}
        validate(data)
        encoded = json.dumps(data, sort_keys=True, separators=(",", ":")).encode()
        if len(encoded) > MAX_STATE:
            raise FixtureError("fixture.invalid_state")
        ordinary(self.directory, directory=True)
        ordinary(self.path, missing=initialize)
        if not initialize:
            self.read()  # refuse a corrupt existing fixture rather than overwrite it
        ordinary(self.pending, missing=True)
        if self.pending.exists():
            if self.pending.stat().st_size > MAX_STATE:
                raise FixtureError("fixture.invalid_state")
            # Under exclusive writer lock, recover only this fixed local scratch
            # file. It never supplies auth state and is never promoted on startup.
            self.pending.unlink()
        fd = os.open(self.pending, os.O_WRONLY | os.O_CREAT | os.O_EXCL | getattr(os, "O_NOFOLLOW", 0), 0o600)
        with os.fdopen(fd, "wb") as file:
            file.write(encoded)
            file.flush()
            os.fsync(file.fileno())
        os.replace(self.pending, self.path)
        if os.name != "nt":
            fd = os.open(self.directory, os.O_RDONLY)
            try:
                os.fsync(fd)
            finally:
                os.close(fd)

    def close(self):
        if self.fd is not None:
            os.close(self.fd)
            self.fd = None


class PersistentRegistry(Registry):
    def __init__(self, directory):
        super().__init__()
        self.store = Store(directory)
        try:
            self.states = self.store.read()
        except Exception:
            self.store.close()
            raise
        self.blocked = False

    def control(self, operation, owner):
        with self.lock:
            if self.blocked:
                return {"ok": False, "code": "fixture.blocked"}
            if operation not in ("rotate", "revoke") or owner not in self.states:
                return {"ok": False, "code": "probe.invalid_local_command"}
            candidate = copy.deepcopy(self.states)
            item = candidate[owner]
            if item["epoch"] == 65535:
                return {"ok": False, "code": "fixture.epoch_limit"}
            if operation == "rotate":
                if item["version"] == 1:
                    return {"ok": False, "code": "probe.rotation_limit"}
                item["version"], item["active"] = 1, True
            else:
                item["active"] = False
            item["epoch"] += 1
            try:
                self.store.write(candidate)
            except Exception:
                # Do not ACK an uncertain write; deny this live instance until
                # operator restart/inspection. Never format exception strings.
                self.blocked = True
                for state in self.states.values():
                    state["active"] = False
                return {"ok": False, "code": "fixture.commit_failed_blocked"}
            self.states = candidate
            return {"ok": True, "operation": operation, "owner": owner,
                    "epoch": item["epoch"], "active": item["active"], "persisted": True}

    def close(self):
        with self.lock:
            self.blocked = True
            for state in self.states.values():
                state["active"] = False
            self.store.close()
