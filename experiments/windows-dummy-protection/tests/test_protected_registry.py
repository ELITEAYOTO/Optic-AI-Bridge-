from pathlib import Path
import subprocess
import copy
import json
import sys
import tempfile
import unittest
from unittest.mock import patch

from protected_registry import ProtectedStore, ProtectedRegistry, FixtureError, _native, MAGIC
from dummy_dpapi import MAX_BLOB, protect_public_fixture, MAGIC as FIXTURE_MAGIC
from lifecycle_core import TOKENS


def bearer(version=0):
    return b'Bearer ' + TOKENS['A'][version]


@unittest.skipUnless(sys.platform == 'win32', 'native protected registry requires Windows')
class ProtectedTests(unittest.TestCase):
    def test_relabelled_fixture_blob_is_not_a_registry_and_is_not_reset(self):
        # Relabel only the public outer prefix; native purpose remains fixture.
        blob = MAGIC + protect_public_fixture()[len(FIXTURE_MAGIC):]
        path = self.root / 'state.protected'
        path.write_bytes(blob)
        with self.assertRaises(FixtureError):
            ProtectedRegistry(self.root)
        self.assertEqual(path.read_bytes(), blob)
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

    def test_valid_disk_drift_is_not_overwritten_by_live_registry(self):
        state = ProtectedRegistry(self.root)
        candidate = copy.deepcopy(state.states)
        candidate['B'].update(epoch=2, active=False)
        blob = MAGIC + _native(json.dumps({'schema': 1, 'owners': candidate}).encode(), True)
        state.store.path.write_bytes(blob)  # controlled owned-fixture replacement
        result = state.control('rotate', 'A')
        self.assertEqual(result, {'ok': False, 'code': 'fixture.commit_failed_blocked'})
        self.assertEqual(state.store.path.read_bytes(), blob)
        self.assertIsNone(state.authenticate(bearer()))
        state.close()
        reopened = ProtectedRegistry(self.root)
        try:
            self.assertIsNone(reopened.authenticate(b'Bearer ' + TOKENS['B'][0]))
            self.assertIsNotNone(reopened.authenticate(bearer()))
        finally:
            reopened.close()

    def test_protected_unreachable_schema_is_rejected_without_reset(self):
        state = ProtectedRegistry(self.root)
        candidate = copy.deepcopy(state.states)
        state.close()
        candidate['A']['epoch'] = 2  # active version0/epoch2 cannot be produced
        blob = MAGIC + _native(json.dumps({'schema': 1, 'owners': candidate}).encode(), True)
        path = self.root / 'state.protected'
        path.write_bytes(blob)
        with self.assertRaises(FixtureError):
            ProtectedRegistry(self.root)
        self.assertEqual(path.read_bytes(), blob)

    def test_oversized_ciphertext_is_rejected_before_native_api(self):
        path = self.root / 'state.protected'
        path.write_bytes(MAGIC + b'x' * MAX_BLOB)
        with patch('protected_registry._native', side_effect=AssertionError('must not decrypt')) as native:
            with self.assertRaises(FixtureError):
                ProtectedRegistry(self.root)
            native.assert_not_called()

    def run_child(self, body):
        # Only fixed test source and owned fixture path; no shell/PID selector.
        source = '''import os, sys
from protected_registry import ProtectedRegistry
from lifecycle_core import TOKENS
state = ProtectedRegistry(sys.argv[1])
''' + body
        child = subprocess.run([sys.executable, '-c', source, str(self.root)],
                               capture_output=True, text=True, timeout=10)
        self.assertEqual(child.returncode, 77, 'fixed fixture child did not reach expected exit')
        self.assertEqual(child.stderr, '', 'child diagnostics suppressed from assertion')
        return child.stdout.strip()

    def test_acknowledged_rotation_and_revoke_survive_abrupt_child_exit(self):
        state = ProtectedRegistry(self.root)
        principal = state.authenticate(bearer()).principal
        state.close()
        output = self.run_child('''assert state.control('rotate', 'A')['persisted']
print('DUMMY COMMIT ACK', flush=True)
os._exit(77)
''')
        self.assertEqual(output, 'DUMMY COMMIT ACK')
        state = ProtectedRegistry(self.root)
        self.assertIsNone(state.authenticate(bearer()))
        identity = state.authenticate(bearer(1))
        self.assertEqual((identity.principal, identity.epoch), (principal, 2))
        state.close()
        self.assertEqual(self.run_child('''assert state.control('revoke', 'A')['persisted']
print('DUMMY COMMIT ACK', flush=True)
os._exit(77)
'''), 'DUMMY COMMIT ACK')
        state = ProtectedRegistry(self.root)
        try:
            self.assertIsNone(state.authenticate(bearer()))
            self.assertIsNone(state.authenticate(bearer(1)))
            self.assertIsNotNone(state.authenticate(b'Bearer ' + TOKENS['B'][0]))
        finally:
            state.close()

    def test_abrupt_exit_before_replace_never_promotes_pending_state(self):
        old = (self.root / 'state.protected').read_bytes()
        self.assertEqual(self.run_child('''import protected_registry
def stop_before_replace(*args):
    os._exit(77)
protected_registry.os.replace = stop_before_replace
state.control('rotate', 'A')
'''), '')
        self.assertTrue((self.root / 'state.protected.pending').exists())
        self.assertEqual((self.root / 'state.protected').read_bytes(), old)
        state = ProtectedRegistry(self.root)
        try:
            self.assertIsNotNone(state.authenticate(bearer()))
            self.assertIsNone(state.authenticate(bearer(1)))
            # A subsequent explicit commit may discard fixed scratch under lock.
            self.assertTrue(state.control('rotate', 'A')['persisted'])
            self.assertFalse(state.store.pending.exists())
        finally:
            state.close()
