from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

from protected_registry import ProtectedStore, ProtectedRegistry, FixtureError, _native
from lifecycle_core import TOKENS


def bearer(version=0):
    return b'Bearer ' + TOKENS['A'][version]


@unittest.skipUnless(sys.platform == 'win32', 'native protected registry requires Windows')
class ProtectedTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name) / 'public-dummy-fixture'
        ProtectedStore.initialize(self.root)

    def tearDown(self):
        self.temp.cleanup()

    def test_rotation_revoke_reopen_and_ciphertext_only(self):
        state = ProtectedRegistry(self.root)
        identity = state.authenticate(bearer())
        self.assertTrue(state.control('rotate', 'A')['persisted'])
        state.close()
        state = ProtectedRegistry(self.root)
        self.assertIsNone(state.authenticate(bearer()))
        self.assertEqual(state.authenticate(bearer(1)).principal, identity.principal)
        self.assertTrue(state.control('revoke', 'A')['persisted'])
        state.close()
        state = ProtectedRegistry(self.root)
        try:
            self.assertIsNone(state.authenticate(bearer(1)))
            self.assertIsNotNone(state.authenticate(b'Bearer ' + TOKENS['B'][0]))
            blob = state.store.path.read_bytes()
            self.assertNotIn(b'"owners"', blob)
            for values in TOKENS.values():
                for token in values:
                    self.assertNotIn(token, blob)
            self.assertFalse((self.root / 'state.json').exists())
        finally:
            state.close()

    def test_replace_failure_blocks_without_ack_and_pending_not_promoted(self):
        state = ProtectedRegistry(self.root)
        with patch('protected_registry.os.replace', side_effect=OSError('SYNTHETIC-CANARY')):
            result = state.control('rotate', 'A')
        self.assertEqual(result, {'ok': False, 'code': 'fixture.commit_failed_blocked'})
        self.assertIsNone(state.authenticate(bearer()))
        state.close()
        reopened = ProtectedRegistry(self.root)
        try:
            self.assertIsNotNone(reopened.authenticate(bearer()))
            self.assertIsNone(reopened.authenticate(bearer(1)))
        finally:
            reopened.close()

    def test_corruption_is_not_reset_or_overwritten(self):
        path = self.root / 'state.protected'
        raw = bytearray(path.read_bytes())
        raw[-1] ^= 1
        path.write_bytes(raw)
        with self.assertRaises(FixtureError):
            ProtectedRegistry(self.root)
        self.assertEqual(path.read_bytes(), bytes(raw))
        with self.assertRaises(FixtureError):
            ProtectedStore.initialize(self.root)

    def test_protection_failure_blocks_and_leaves_committed_bytes(self):
        state = ProtectedRegistry(self.root)
        old = state.store.path.read_bytes()
        def fail_protection(data, protect):
            if protect:
                raise RuntimeError('SYNTHETIC-CANARY')
            return _native(data, protect)
        with patch('protected_registry._native', side_effect=fail_protection):
            result = state.control('rotate', 'A')
        self.assertEqual(result, {'ok': False, 'code': 'fixture.commit_failed_blocked'})
        self.assertIsNone(state.authenticate(bearer()))
        self.assertEqual(state.store.path.read_bytes(), old)
        state.close()

    def test_exclusive_writer_and_missing_state_fail_closed(self):
        state = ProtectedRegistry(self.root)
        with self.assertRaises(FixtureError):
            ProtectedRegistry(self.root)
        state.close()
        path = self.root / 'state.protected'
        path.unlink()
        with self.assertRaises(FixtureError):
            ProtectedRegistry(self.root)
        self.assertFalse(path.exists())

    def test_valid_old_ciphertext_replay_is_explicitly_not_prevented(self):
        path = self.root / 'state.protected'
        old = path.read_bytes()
        state = ProtectedRegistry(self.root)
        self.assertTrue(state.control('revoke', 'A')['persisted'])
        state.close()
        path.write_bytes(old)  # deliberate owned-fixture rollback characterization
        state = ProtectedRegistry(self.root)
        try:
            self.assertIsNotNone(state.authenticate(bearer()))
        finally:
            state.close()
