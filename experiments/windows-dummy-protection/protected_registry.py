"""Disposable public-dummy metadata integration. No server or real tokens."""
import copy
import json
import os
import sys

from dummy_dpapi import _native as _dpapi_native, MAX_BLOB
from lifecycle_core import Registry
from persistent_core import Store, PersistentRegistry, FixtureError, validate, unique_object, ordinary, MAX_STATE

MAGIC = b'OPTIC-PUBLIC-DUMMY-REGISTRY-V1\0'


def _native(data, protect):
    return _dpapi_native(data, protect, purpose='registry')


def decode_metadata(raw):
    """Exact finite dummy schema, never a permissive protected-state parser."""
    if not isinstance(raw, bytes) or not 0 < len(raw) <= MAX_STATE:
        raise FixtureError('fixture.invalid_state')
    try:
        owners = validate(json.loads(raw, object_pairs_hook=unique_object))
        # With version0, any revocation is terminal until rotation advances to1.
        # No allowed local operation can produce active version0 at epoch>1.
        if any(value['version'] == 0 and value['active'] and value['epoch'] != 1
               for value in owners.values()):
            raise FixtureError('fixture.invalid_state')
        return copy.deepcopy(owners)
    except (ValueError, UnicodeError, RecursionError):
        raise FixtureError('fixture.invalid_state') from None


class ProtectedStore(Store):
    def __init__(self, directory):
        if sys.platform != 'win32':
            raise FixtureError('fixture.windows_required')
        super().__init__(directory)
        self.path = self.directory / 'state.protected'
        self.pending = self.directory / 'state.protected.pending'
        self.expected = None

    def read(self):
        ordinary(self.directory, directory=True)
        ordinary(self.path)
        with self.path.open('rb') as file:
            blob = file.read(MAX_BLOB + 1)
        if not len(MAGIC) < len(blob) <= MAX_BLOB or not blob.startswith(MAGIC):
            raise FixtureError('fixture.invalid_state')
        try:
            raw = _native(blob[len(MAGIC):], False)
            owners = decode_metadata(raw)
            if self.expected is None:
                self.expected = copy.deepcopy(owners)
            return owners
        except Exception:
            raise FixtureError('fixture.invalid_state') from None

    def write(self, owners, initialize=False):
        data = {'schema': 1, 'owners': owners}
        validate(data)  # only bounded public-dummy metadata; no arbitrary payload
        raw = json.dumps(data, sort_keys=True, separators=(',', ':')).encode('ascii')
        decode_metadata(raw)
        ordinary(self.directory, directory=True)
        ordinary(self.path, missing=initialize)
        if initialize and self.path.exists():
            raise FixtureError('fixture.already_exists')
        if not initialize:
            current = self.read()
            if current != self.expected:
                raise FixtureError('fixture.state_changed')
        blob = MAGIC + _native(raw, True)
        if len(blob) > MAX_BLOB:
            raise FixtureError('fixture.invalid_state')
        ordinary(self.pending, missing=True)
        if self.pending.exists():
            if self.pending.stat().st_size > MAX_BLOB:
                raise FixtureError('fixture.invalid_state')
            self.pending.unlink()
        fd = os.open(self.pending, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
        with os.fdopen(fd, 'wb') as file:
            file.write(blob)
            file.flush()
            os.fsync(file.fileno())
        os.replace(self.pending, self.path)
        self.expected = copy.deepcopy(owners)


class ProtectedRegistry(PersistentRegistry):
    def __init__(self, directory):
        Registry.__init__(self)
        self.store = ProtectedStore(directory)
        try:
            self.states = self.store.read()
        except Exception:
            self.store.close()
            raise
        self.blocked = False
