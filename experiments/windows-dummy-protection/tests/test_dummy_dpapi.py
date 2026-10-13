import contextlib
import io
from pathlib import Path
import sys
import subprocess
import tempfile
import unittest
from unittest.mock import patch

from dummy_dpapi import (check_public_fixture, protect_public_fixture, FixtureError,
                         PAYLOAD, MAGIC, MAX_BLOB, UI_FORBIDDEN, PUBLIC_DUMMIES)


class BoundaryTests(unittest.TestCase):
    def test_invalid_input_never_reaches_native(self):
        with patch('dummy_dpapi._native', side_effect=AssertionError('native should not run')):
            for blob in (None, {}, 'secret-canary', b'', MAGIC, MAGIC + b'x' * MAX_BLOB, b'unknown'):
                with self.assertRaises(FixtureError) as error:
                    check_public_fixture(blob)
                self.assertEqual(str(error.exception), 'dummy_dpapi.invalid_fixture')

    def test_unexpected_plaintext_is_not_returned_or_echoed(self):
        capture = io.StringIO()
        with contextlib.redirect_stdout(capture), patch('dummy_dpapi._native', return_value=b'SYNTHETIC-CANARY'):
            with self.assertRaises(FixtureError) as error:
                check_public_fixture(MAGIC + b'synthetic')
        self.assertEqual(str(error.exception), 'dummy_dpapi.unexpected_payload')
        self.assertEqual(capture.getvalue(), '')

    def test_public_verifier_fixture_and_flag(self):
        self.assertEqual(UI_FORBIDDEN, 1)
        self.assertLess(len(PAYLOAD), 4096)
        for raw in PUBLIC_DUMMIES:
            self.assertNotIn(raw, PAYLOAD)

    @unittest.skipIf(sys.platform == 'win32', 'non-Windows fail-closed branch')
    def test_non_windows_has_no_plaintext_fallback(self):
        with self.assertRaises(FixtureError) as error:
            protect_public_fixture()
        self.assertEqual(str(error.exception), 'dummy_dpapi.windows_required')


@unittest.skipUnless(sys.platform == 'win32', 'native CurrentUser proof requires Windows')
class NativeTests(unittest.TestCase):
    def test_real_current_user_roundtrip_is_silent_and_no_plaintext_storage(self):
        capture = io.StringIO()
        with contextlib.redirect_stdout(capture):
            blob = protect_public_fixture()
            result = check_public_fixture(blob)
        self.assertTrue(result['ok'])
        self.assertLessEqual(len(blob), MAX_BLOB)
        self.assertEqual(capture.getvalue(), '')
        self.assertNotIn(PAYLOAD, blob)
        for raw in PUBLIC_DUMMIES:
            self.assertNotIn(raw, blob)
        # Disposable fixture file only; no actual configuration or key-store paths.
        with tempfile.TemporaryDirectory(prefix='optic-public-dummy-dpapi-') as directory:
            fixture = Path(directory) / 'dummy_fixture.bin'
            fixture.write_bytes(blob)
            self.assertTrue(check_public_fixture(fixture.read_bytes())['ok'])

    def test_real_native_corrupted_ciphertext_is_rejected_without_values(self):
        blob = bytearray(protect_public_fixture())
        blob[-1] ^= 1
        with self.assertRaises(FixtureError) as error:
            check_public_fixture(bytes(blob))
        self.assertIn(str(error.exception), ('dummy_dpapi.unprotect_failed', 'dummy_dpapi.unexpected_payload',
                                            'dummy_dpapi.output_bound'))

    def test_native_protection_randomizes_same_public_fixture(self):
        first, second = protect_public_fixture(), protect_public_fixture()
        self.assertNotEqual(first, second)
        self.assertTrue(check_public_fixture(first)['ok'])
        self.assertTrue(check_public_fixture(second)['ok'])

    def test_fixture_from_fresh_process_same_user_verifies(self):
        source = Path(__file__).resolve().parents[1]
        child_code = '''import sys
from pathlib import Path
sys.path.insert(0, sys.argv[1])
from dummy_dpapi import protect_public_fixture
try:
    Path(sys.argv[2]).write_bytes(protect_public_fixture())
except Exception:
    print('DUMMY DPAPI CHILD FAILED')
    raise SystemExit(1)
print('DUMMY DPAPI CHILD PASS')
'''
        with tempfile.TemporaryDirectory(prefix='optic-public-dummy-dpapi-') as directory:
            fixture = Path(directory) / 'dummy_fixture.bin'
            child = subprocess.run([sys.executable, '-c', child_code, str(source), str(fixture)],
                                   capture_output=True, text=True, timeout=10)
            self.assertEqual(child.returncode, 0, 'native child failed; no raw diagnostics echoed')
            self.assertEqual(child.stdout.strip(), 'DUMMY DPAPI CHILD PASS')
            self.assertEqual(child.stderr, '')
            blob = fixture.read_bytes()
            self.assertTrue(check_public_fixture(blob)['ok'])


if __name__ == '__main__':
    unittest.main()
