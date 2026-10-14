import ctypes
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

from native_fixture_lock import PreparedFixtureLock
from native_fixture_creation import create_private_fixture
from native_fixture_read import PreparedFixtureRead
from native_fixture_security import observe_fixture_security
from native_fixture_pending import PendingError, PUBLIC_PENDING
from native_relative_pending import RootRelativeFixturePending, ObjectAttributes, UnicodeName, IoStatus


class Bounds(unittest.TestCase):
    def test_unopened_pin_never_loads_ntdll(self):
        with patch.object(ctypes, 'WinDLL', side_effect=AssertionError('must not load'), create=True):
            with self.assertRaises(PendingError):
                with RootRelativeFixturePending(PreparedFixtureLock('C:\\public-dummy')):
                    self.fail('unopened pin admitted')


@unittest.skipUnless(sys.platform == 'win32', 'native Windows x64 relative-create proof')
class Native(unittest.TestCase):
    def test_fresh_private_root_lock_and_relative_scratch_acl_composition(self):
        with tempfile.TemporaryDirectory(prefix='optic-public-composition-') as directory:
            (Path(directory) / 'state.protected').write_bytes(b'PUBLIC PARENT ONLY')
            with PreparedFixtureRead(directory) as parent:
                root = create_private_fixture(parent)
                (root / 'writer.lock').write_bytes(b'0')
                with PreparedFixtureLock(root) as pin:
                    lock_security = observe_fixture_security(pin.prepared)
                    with RootRelativeFixturePending(pin) as pending:
                        pending.write_once()
                        # Borrow the live handle solely for the read-only observer.
                        # Restore ownership before any context cleanup, including failures.
                        lock_handle = pin.prepared.file
                        try:
                            pin.prepared.file = pending.file
                            scratch_security = observe_fixture_security(pin.prepared)
                        finally:
                            pin.prepared.file = lock_handle
                        self.assertEqual(lock_security['root'], scratch_security['root'])
                        self.assertTrue(scratch_security['root']['dacl_protected'])
                        self.assertTrue(scratch_security['root']['owner_matches_process_user'])
                        for row in (lock_security['file'], scratch_security['file']):
                            self.assertTrue(row['dacl_present'])
                            self.assertFalse(row['null_dacl'])
                            self.assertEqual(row['ace_count'], 3)
                            self.assertEqual(row['aces']['allow_write_aces']['other'], 0)
                            self.assertEqual(row['aces']['unparsed_aces'], 0)
                self.assertEqual((root / 'writer.lock').read_bytes(), b'0')
                self.assertEqual((root / 'state.protected.pending').read_bytes(), PUBLIC_PENDING)
                self.assertFalse((root / 'state.protected').exists())

    def test_unsupported_platform_never_creates_or_falls_back(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'writer.lock').write_bytes(b'0')
            with PreparedFixtureLock(directory) as pin:
                with patch.object(sys, 'platform', 'unsupported'):
                    with patch.object(ctypes, 'WinDLL', side_effect=AssertionError('must not load')):
                        with self.assertRaises(PendingError):
                            with RootRelativeFixturePending(pin):
                                self.fail('unsupported platform admitted')
            self.assertFalse((root / 'state.protected.pending').exists())

    def test_unexpected_native_completion_closes_only_success_owned_handle(self):
        for status, completion_status, information, expected_closes in (
                (0, 0, 1, [101]), (1, 0, 2, [101]), (0, 1, 2, [101]), (-1, 0, 2, [])):
            with tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                (root / 'writer.lock').write_bytes(b'0')
                with PreparedFixtureLock(directory) as pin:
                    class Create:
                        def __call__(self, output, access, attributes, completion, *rest):
                            packet = ctypes.cast(attributes, ctypes.POINTER(ObjectAttributes)).contents
                            self_root = packet.root
                            name = packet.name.contents
                            assert self_root == pin.prepared.root
                            assert ctypes.wstring_at(name.buffer, name.length // 2) == 'state.protected.pending'
                            ctypes.cast(output, ctypes.POINTER(ctypes.c_void_p)).contents.value = 101  # simulated only
                            io = ctypes.cast(completion, ctypes.POINTER(IoStatus)).contents
                            io.result.status, io.information = completion_status, information
                            return status
                    class NativeLibrary:
                        NtCreateFile = Create()
                    calls = []
                    with patch.object(ctypes, 'WinDLL', return_value=NativeLibrary()):
                        with patch.object(pin.prepared.kernel, 'CloseHandle', side_effect=lambda value: calls.append(value) or 1):
                            with self.assertRaises(PendingError):
                                with RootRelativeFixturePending(pin):
                                    self.fail('ambiguous completion admitted')
                    self.assertEqual(calls, expected_closes)
                self.assertFalse((root / 'state.protected.pending').exists())

    def test_actual_relative_create_without_absolute_create_api(self):
        self.assertEqual((ctypes.sizeof(UnicodeName), ctypes.sizeof(ObjectAttributes), ctypes.sizeof(IoStatus)), (16, 48, 16))
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'writer.lock').write_bytes(b'0')
            with PreparedFixtureLock(directory) as pin:
                with patch.object(pin.prepared.kernel, 'CreateFileW', side_effect=AssertionError('absolute create called')):
                    with RootRelativeFixturePending(pin) as pending:
                        pending.write_once()
                        with self.assertRaises(OSError):
                            (root / 'state.protected.pending').write_bytes(b'x')
            self.assertEqual((root / 'state.protected.pending').read_bytes(), PUBLIC_PENDING)
            self.assertFalse((root / 'state.protected').exists())

    def test_existing_target_is_preserved_and_never_falls_back(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'writer.lock').write_bytes(b'0')
            target = root / 'state.protected.pending'
            target.write_bytes(b'PUBLIC EXISTING RELATIVE SCRATCH')
            with PreparedFixtureLock(directory) as pin:
                with patch.object(pin.prepared.kernel, 'CreateFileW', side_effect=AssertionError('fallback called')):
                    with self.assertRaises(PendingError):
                        with RootRelativeFixturePending(pin):
                            self.fail('existing target admitted')
            self.assertEqual(target.read_bytes(), b'PUBLIC EXISTING RELATIVE SCRATCH')

    def test_creation_ignores_mutated_path_string_and_uses_open_root(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            original, decoy = root / 'original', root / 'decoy'
            original.mkdir()
            decoy.mkdir()
            (original / 'writer.lock').write_bytes(b'0')
            with PreparedFixtureLock(str(original)) as pin:
                pin.prepared.directory = str(decoy)
                with RootRelativeFixturePending(pin) as pending:
                    pending.write_once()
            self.assertEqual((original / 'state.protected.pending').read_bytes(), PUBLIC_PENDING)
            self.assertFalse((decoy / 'state.protected.pending').exists())

    def test_native_loader_failure_preserves_absence_without_fallback(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'writer.lock').write_bytes(b'0')
            with PreparedFixtureLock(directory) as pin:
                with patch.object(ctypes, 'WinDLL', side_effect=OSError('PUBLIC TEST')):
                    with self.assertRaises(PendingError):
                        with RootRelativeFixturePending(pin):
                            self.fail('loader failure admitted')
            self.assertFalse((root / 'state.protected.pending').exists())
