import ctypes
import os
from pathlib import Path
import sys
import tempfile
from types import SimpleNamespace
import unittest

from native_fixture_lock import PreparedFixtureLock
from native_fixture_pending import PreparedFixturePending, PendingError, PUBLIC_PENDING


class Bounds(unittest.TestCase):
    def test_full_write_with_failed_flush_stays_failed_and_never_retries(self):
        calls = []
        class Kernel:
            def WriteFile(self, file, buffer, size, output, overlap):
                calls.append('write')
                ctypes.cast(output, ctypes.POINTER(ctypes.c_uint32)).contents.value = size
                return 1
            def FlushFileBuffers(self, file):
                calls.append('flush')
                return 0
        pin = PreparedFixtureLock('C:\\public-dummy')
        item = pin.prepared
        item.kernel, item.root, item.file = Kernel(), 1, 2
        item._final = lambda handle: 'C:\\public-dummy' if handle == 1 else 'C:\\public-dummy\\state.protected.pending'
        pending = PreparedFixturePending(pin)
        pending.file, pending.kernel, pending.root_name = 3, item.kernel, 'C:\\public-dummy'
        with self.assertRaises(PendingError):
            pending.write_once()
        self.assertEqual(calls, ['write', 'flush'])
        self.assertTrue(pending.failed)
        with self.assertRaises(PendingError):
            pending.write_once()
        self.assertEqual(calls, ['write', 'flush'])

    def test_unopened_or_unknown_lock_denies_before_native_creation(self):
        for pin in (None, {}, PreparedFixtureLock('C:\\public-dummy')):
            with self.assertRaises(PendingError):
                with PreparedFixturePending(pin):
                    self.fail('unopened pin admitted')

    def test_partial_or_invalid_write_count_never_flushes_or_retries(self):
        for amount in (0, len(PUBLIC_PENDING) - 1, len(PUBLIC_PENDING) + 1, 0xffffffff):
            flushed = []
            class Kernel:
                def WriteFile(self, file, buffer, size, output, overlap):
                    ctypes.cast(output, ctypes.POINTER(ctypes.c_uint32)).contents.value = amount
                    return 1
                def FlushFileBuffers(self, file):
                    flushed.append(file)
                    return 1
            pin = PreparedFixtureLock('C:\\public-dummy')
            item = pin.prepared
            item.kernel = Kernel()
            item.root, item.file = 1, 2
            item._final = lambda handle: 'C:\\public-dummy' if handle == 1 else 'C:\\public-dummy\\state.protected.pending'
            pending = PreparedFixturePending(pin)
            pending.file, pending.kernel, pending.root_name = 3, item.kernel, 'C:\\public-dummy'
            with self.assertRaises(PendingError):
                pending.write_once()
            self.assertEqual(flushed, [])
            self.assertTrue(pending.failed)
            with self.assertRaises(PendingError):
                pending.write_once()

    def test_failed_close_is_sticky_without_retry(self):
        calls = []
        pending = PreparedFixturePending(None)
        pending.file = 4
        pending.kernel = SimpleNamespace(CloseHandle=lambda handle: calls.append(handle) or 0)
        with self.assertRaises(PendingError):
            pending.close()
        with self.assertRaises(PendingError):
            pending.close()
        self.assertEqual(calls, [4])


@unittest.skipUnless(sys.platform == 'win32', 'native Windows scratch-file proof')
class Native(unittest.TestCase):
    def test_existing_file_symlink_refused_without_modifying_target(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'writer.lock').write_bytes(b'0')
            original = root / 'public-target'
            original.write_bytes(b'PUBLIC SYMLINK TARGET')
            target = root / 'state.protected.pending'
            try:
                target.symlink_to(original)
            except OSError:
                self.skipTest('native symlink creation not permitted')
            with PreparedFixtureLock(directory) as pin:
                with self.assertRaises(PendingError):
                    with PreparedFixturePending(pin):
                        self.fail('existing symlink admitted')
            self.assertTrue(target.is_symlink())
            self.assertEqual(original.read_bytes(), b'PUBLIC SYMLINK TARGET')

    def test_create_write_flush_pin_and_no_state_promotion(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'writer.lock').write_bytes(b'0')
            state = root / 'state.protected'
            state.write_bytes(b'PUBLIC COMMITTED STATE')
            target = root / 'state.protected.pending'
            with PreparedFixtureLock(directory) as pin:
                with PreparedFixturePending(pin) as pending:
                    pending.write_once()
                    with self.assertRaises(PendingError):
                        pending.write_once()
                    with self.assertRaises(PendingError):
                        pending.__enter__()
                    for operation in (lambda: target.write_bytes(b'x'), target.unlink,
                                      lambda: target.rename(root / 'moved.pending')):
                        with self.assertRaises(OSError):
                            operation()
                    with self.assertRaises(PendingError):
                        with PreparedFixturePending(pin):
                            self.fail('second scratch creation admitted')
                self.assertEqual(target.read_bytes(), PUBLIC_PENDING)
            self.assertEqual(state.read_bytes(), b'PUBLIC COMMITTED STATE')

    def test_existing_file_directory_or_link_is_never_repaired(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'writer.lock').write_bytes(b'0')
            target = root / 'state.protected.pending'
            target.write_bytes(b'PUBLIC EXISTING SCRATCH')
            with PreparedFixtureLock(directory) as pin:
                with self.assertRaises(PendingError):
                    with PreparedFixturePending(pin):
                        pass
                self.assertEqual(target.read_bytes(), b'PUBLIC EXISTING SCRATCH')
                linked = root / 'hardlink'
                os.link(target, linked)
                with self.assertRaises(PendingError):
                    with PreparedFixturePending(pin):
                        pass
                self.assertEqual(linked.read_bytes(), b'PUBLIC EXISTING SCRATCH')
                target.unlink()
                target.mkdir()
                with self.assertRaises(PendingError):
                    with PreparedFixturePending(pin):
                        pass
                self.assertTrue(target.is_dir())

    def test_closed_lock_cannot_create_or_write(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'writer.lock').write_bytes(b'0')
            pin = PreparedFixtureLock(directory)
            with pin:
                pending = PreparedFixturePending(pin)
                pending.__enter__()
            try:
                with self.assertRaises(PendingError):
                    pending.write_once()
            finally:
                pending.close()
            self.assertEqual((root / 'state.protected.pending').read_bytes(), b'')
            (root / 'state.protected.pending').unlink()
            with self.assertRaises(PendingError):
                with PreparedFixturePending(pin):
                    pass
            self.assertFalse((root / 'state.protected.pending').exists())
