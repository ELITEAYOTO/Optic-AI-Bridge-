"""Negative security characterization, not an approved replacement primitive."""
import copy
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

from lifecycle_core import TOKENS
from native_fixture_lock import PreparedFixtureLock
from native_fixture_read import PreparedFixtureRead
from native_fixture_pending import PendingError, PUBLIC_PENDING
from native_initial_publication import _InitialPending, RenamePacket
from protected_registry import ProtectedStore, ProtectedRegistry, _native, MAGIC


@unittest.skipUnless(sys.platform == 'win32', 'native replacement gap characterization')
class NativeGap(unittest.TestCase):
    def test_held_read_pin_blocks_replace_but_release_gap_allows_lost_write(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            target = root / 'state.protected'
            target.write_bytes(b'PUBLIC EXPECTED OLD')
            (root / 'writer.lock').write_bytes(b'0')
            with PreparedFixtureLock(root) as writer:
                with _InitialPending(writer) as pending:
                    pending.write_once()
                    # Replace enabled ONLY in this negative characterization.
                    # Public fixture publisher still uses zero replace flags.
                    packet = RenamePacket(1, writer.prepared.root, 30, 'state.protected')
                    with PreparedFixtureRead(root) as current:
                        expected = current.read()
                        self.assertEqual(expected, b'PUBLIC EXPECTED OLD')
                        with self.assertRaises(OSError):
                            target.write_bytes(b'PUBLIC OUTSIDE WRITER')
                        with self.assertRaises(PendingError):
                            pending._rename(packet)
                    # The cooperating writer lock is still held, but another
                    # same-user writer ignoring that lock can now alter state.
                    target.write_bytes(b'PUBLIC OUTSIDE WRITER')
                    self.assertNotEqual(target.read_bytes(), expected)
                    pending._rename(packet)
                    self.assertTrue(writer.prepared.file is not None)
            self.assertEqual(target.read_bytes(), PUBLIC_PENDING)
            self.assertFalse((root / 'state.protected.pending').exists())

    def test_protected_fixture_precheck_does_not_prevent_later_revocation_loss(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / 'public-fixture'
            ProtectedStore.initialize(root)
            state = ProtectedRegistry(root)
            try:
                changed = copy.deepcopy(state.states)
                changed['B']['active'] = False
                changed['B']['epoch'] += 1
                raw = json.dumps({'schema': 1, 'owners': changed},
                                 sort_keys=True, separators=(',', ':')).encode('ascii')
                outside_blob = MAGIC + _native(raw, True)
                observed = []
                def interleave(data, protect):
                    if protect:
                        # write() already checked current == expected and closed
                        # the state read pin before reaching encryption.
                        state.store.path.write_bytes(outside_blob)
                        observed.append(state.store.read()['B']['active'])
                    return _native(data, protect)
                with patch('protected_registry._native', side_effect=interleave):
                    result = state.control('rotate', 'A')
                self.assertEqual(observed, [False])
                self.assertTrue(result['persisted'])
            finally:
                state.close()
            reopened = ProtectedRegistry(root)
            try:
                # Measured limitation: the outside B revocation was overwritten.
                self.assertIsNotNone(reopened.authenticate(b'Bearer ' + TOKENS['B'][0]))
                self.assertIsNotNone(reopened.authenticate(b'Bearer ' + TOKENS['A'][1]))
            finally:
                reopened.close()
