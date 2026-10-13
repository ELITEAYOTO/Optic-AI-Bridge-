import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

from native_fixture_lock import PreparedFixtureLock, LockError


class Bounds(unittest.TestCase):
    def test_close_failure_is_sticky_and_attempts_both_handles(self):
        lock = PreparedFixtureLock('C:\\public-dummy')
        calls = []
        class Kernel:
            def CloseHandle(self, handle):
                calls.append(handle)
                return handle != 1
        lock.prepared.kernel = Kernel()
        lock.prepared.file, lock.prepared.root = 1, 2
        with self.assertRaisesRegex(LockError, 'fixture_lock.close_failed'):
            lock.close()
        self.assertEqual(calls, [1, 2])
        with self.assertRaises(LockError):
            lock.close()
        self.assertEqual(calls, [1, 2])

    @unittest.skipIf(sys.platform == 'win32', 'unsupported-platform proof')
    def test_no_non_windows_fallback_or_file_creation(self):
        with self.assertRaises(LockError):
            with PreparedFixtureLock('C:\\public-dummy'):
                self.fail('unsupported lock admitted')


@unittest.skipUnless(sys.platform == 'win32', 'native Windows lock proof')
class Native(unittest.TestCase):
    def test_file_and_root_symlinks_refused_without_following_when_supported(self):
        with tempfile.TemporaryDirectory() as parent:
            base = Path(parent)
            target = base / 'target'
            target.mkdir()
            public = target / 'public.lock'
            public.write_bytes(b'0')
            file_link = target / 'writer.lock'
            root_link = base / 'root-link'
            try:
                file_link.symlink_to(public)
                root_link.symlink_to(target, target_is_directory=True)
            except OSError:
                self.skipTest('native symlink creation not permitted')
            with self.assertRaises(LockError):
                with PreparedFixtureLock(str(target)):
                    self.fail('file reparse admitted')
            file_link.unlink()
            file_link.write_bytes(b'0')
            with self.assertRaises(LockError):
                with PreparedFixtureLock(str(root_link)):
                    self.fail('root reparse admitted')
            self.assertEqual(public.read_bytes(), b'0')
            self.assertEqual(file_link.read_bytes(), b'0')

    def test_reentry_does_not_release_held_pin(self):
        with tempfile.TemporaryDirectory() as directory:
            (Path(directory) / 'writer.lock').write_bytes(b'0')
            lock = PreparedFixtureLock(directory)
            with lock:
                with self.assertRaisesRegex(LockError, 'fixture_lock.already_opened'):
                    lock.__enter__()
                with self.assertRaises(LockError):
                    with PreparedFixtureLock(directory):
                        self.fail('reentry released original pin')

    def test_exclusive_open_and_release(self):
        with tempfile.TemporaryDirectory() as directory:
            lock = Path(directory) / 'writer.lock'
            lock.write_bytes(b'0')
            with PreparedFixtureLock(directory):
                with self.assertRaises(LockError):
                    with PreparedFixtureLock(directory):
                        self.fail('second lock admitted')
                for operation in (lambda: lock.write_bytes(b'x'), lock.unlink,
                                  lambda: lock.rename(lock.with_name('renamed.lock'))):
                    with self.assertRaises(OSError):
                        operation()
            self.assertEqual(lock.read_bytes(), b'0')
            with PreparedFixtureLock(directory):
                pass

    def test_fresh_process_refused_then_released(self):
        with tempfile.TemporaryDirectory() as directory:
            (Path(directory) / 'writer.lock').write_bytes(b'0')
            code = ('import sys; from native_fixture_lock import PreparedFixtureLock,LockError; '
                    '\ntry:\n with PreparedFixtureLock(sys.argv[1]): pass'
                    '\nexcept LockError: sys.exit(7)')
            def child():
                return subprocess.run([sys.executable, '-c', code, directory],
                    capture_output=True, timeout=10)
            with PreparedFixtureLock(directory):
                refused = child()
                self.assertEqual(refused.returncode, 7)
                self.assertEqual(refused.stdout + refused.stderr, b'')
            self.assertEqual(child().returncode, 0)

    def test_missing_wrong_marker_and_hardlink_preserved(self):
        with tempfile.TemporaryDirectory() as directory:
            lock = Path(directory) / 'writer.lock'
            with self.assertRaises(LockError):
                with PreparedFixtureLock(directory):
                    pass
            self.assertFalse(lock.exists())
            for value in (b'', b'x', b'00'):
                lock.write_bytes(value)
                with self.assertRaises(LockError):
                    with PreparedFixtureLock(directory):
                        pass
                self.assertEqual(lock.read_bytes(), value)
            lock.write_bytes(b'0')
            other = Path(directory) / 'linked.lock'
            os.link(lock, other)
            with self.assertRaises(LockError):
                with PreparedFixtureLock(directory):
                    pass
            self.assertEqual(lock.read_bytes(), b'0')

    def test_root_rename_denied_and_context_error_releases(self):
        with tempfile.TemporaryDirectory() as parent:
            root = Path(parent) / 'fixture'
            root.mkdir()
            (root / 'writer.lock').write_bytes(b'0')
            with self.assertRaisesRegex(RuntimeError, 'PUBLIC TEST'):
                with PreparedFixtureLock(str(root)):
                    with self.assertRaises(OSError):
                        root.rename(Path(parent) / 'moved')
                    raise RuntimeError('PUBLIC TEST')
            with PreparedFixtureLock(str(root)):
                pass
