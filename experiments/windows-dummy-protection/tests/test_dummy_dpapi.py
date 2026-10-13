import contextlib
import ctypes
import io
from pathlib import Path
import sys
import subprocess
import tempfile
import unittest
from unittest.mock import patch

from dummy_dpapi import (check_public_fixture, protect_public_fixture, FixtureError,
                         PAYLOAD, MAGIC, MAX_BLOB, UI_FORBIDDEN, PUBLIC_DUMMIES)
import dummy_dpapi


class NativeFailureTests(unittest.TestCase):
    def invoke_fake(self, *, succeeded=True, size=4, free_failed=False):
        # Controlled allocation owned by this test; never touch arbitrary memory.
        buffer = (ctypes.c_ubyte * 4)(1, 2, 3, 4)
        freed = []

        class Function:
            def __init__(self, callback):
                self.callback = callback

            def __call__(self, *args):
                return self.callback(*args)

        def transform(*args):
            output = args[-1]._obj
            output.size = size
            output.data = ctypes.cast(buffer, ctypes.POINTER(ctypes.c_ubyte))
            return succeeded

        def release(pointer):
            freed.append(bytes(buffer))
            return 1 if free_failed else None

        class Library:
            pass

        crypt, kernel = Library(), Library()
        crypt.CryptProtectData = Function(transform)
        kernel.LocalFree = Function(release)
        with patch.object(dummy_dpapi.sys, 'platform', 'win32'), patch.object(
                ctypes, 'WinDLL', side_effect=[crypt, kernel], create=True):
            try:
                result = dummy_dpapi._native(PAYLOAD, True)
            except FixtureError as error:
                result = error.code
        return result, freed

    def test_failed_native_call_still_zeroes_and_frees_output(self):
        result, freed = self.invoke_fake(succeeded=False)
        self.assertEqual(result, 'dummy_dpapi.protect_failed')
        self.assertEqual(freed, [b'\0' * 4])

    def test_oversized_output_is_not_copied_or_zeroed_beyond_bound(self):
        with patch.object(ctypes, 'string_at', side_effect=AssertionError('must not copy')):
            result, freed = self.invoke_fake(size=MAX_BLOB + 1)
        self.assertEqual(result, 'dummy_dpapi.output_bound')
        self.assertEqual(freed, [bytes([1, 2, 3, 4])])

    def test_free_failure_prevents_success(self):
        result, freed = self.invoke_fake(free_failed=True)
        self.assertEqual(result, 'dummy_dpapi.native_free_failed')
        self.assertEqual(freed, [b'\0' * 4])

    def test_library_failure_is_fixed_and_has_no_fallback(self):
        with patch.object(dummy_dpapi.sys, 'platform', 'win32'), patch.object(
                ctypes, 'WinDLL', side_effect=OSError('SYNTHETIC-CANARY'), create=True):
            with self.assertRaises(FixtureError) as error:
                protect_public_fixture()
        self.assertEqual(str(error.exception), 'dummy_dpapi.native_unavailable')


class BoundaryTests(unittest.TestCase):
    def test_unknown_purpose_is_rejected_before_native_loading(self):
        with patch.object(ctypes, 'WinDLL', side_effect=AssertionError('must not load'), create=True):
            for purpose in (None, '', 'SYNTHETIC-CANARY', 'machine', b'fixture'):
                with self.assertRaises(FixtureError) as error:
                    dummy_dpapi._native(PAYLOAD, True, purpose=purpose)
                self.assertEqual(str(error.exception), 'dummy_dpapi.invalid_purpose')
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
    def test_same_dummy_payload_cannot_be_unprotected_under_other_purpose(self):
        for source, destination in (('fixture', 'registry'), ('registry', 'fixture')):
            encrypted = dummy_dpapi._native(PAYLOAD, True, purpose=source)
            self.assertEqual(dummy_dpapi._native(encrypted, False, purpose=source), PAYLOAD)
            with self.assertRaises(FixtureError):
                dummy_dpapi._native(encrypted, False, purpose=destination)
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
