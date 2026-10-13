import os
import subprocess
from pathlib import Path
import sys
import tempfile
import unittest

from native_fixture_read import PreparedFixtureRead, ReadError, fixture_directory
from dummy_dpapi import MAX_BLOB


class DirectoryTests(unittest.TestCase):
    def test_close_failure_still_attempts_both_handles_and_denies_success(self):
        prepared = PreparedFixtureRead('C:\\dummy-fixture')
        calls = []
        class FakeKernel:
            def CloseHandle(self, handle):
                calls.append(handle)
                return handle != 101
        prepared.kernel = FakeKernel()
        prepared.file, prepared.root = 101, 202  # simulated handles only
        with self.assertRaises(ReadError) as error:
            prepared.close()
        self.assertEqual(str(error.exception), 'fixture_read.close_failed')
        self.assertEqual(calls, [101, 202])
        with self.assertRaises(ReadError):
            prepared.close()  # uncertain cleanup stays failed; no blind retry
        self.assertEqual(calls, [101, 202])
        self.assertTrue(prepared.close_uncertain)

    def test_invalid_native_read_count_is_not_copied(self):
        for count in (0, 5, 0xffffffff):
            prepared = PreparedFixtureRead('C:\\dummy-fixture')
            class FakeKernel:
                def ReadFile(self, handle, buffer, bound, output, overlapped):
                    self.bound = bound
                    output._obj.value = count
                    return True
            prepared.kernel = FakeKernel()
            prepared.file, prepared.size = 101, 4  # simulated handle/count only
            with self.assertRaises(ReadError) as error:
                prepared.read()
            self.assertEqual(str(error.exception), 'fixture_read.read_changed')
            self.assertEqual(prepared.kernel.bound, MAX_BLOB + 1)
    def test_directory_grammar_rejects_device_unc_relative_and_traversal(self):
        for value in (None, {}, b'C:\\fixture', '', 'fixture', '\\\\server\\share',
                      '\\\\.\\C:', 'C:\\a\\..\\b', 'C:\\a\0b'):
            with self.assertRaises(ReadError):
                fixture_directory(value)
        self.assertEqual(fixture_directory('C:\\dummy-fixture'), 'C:\\dummy-fixture')

    @unittest.skipIf(sys.platform == 'win32', 'unsupported-platform branch')
    def test_non_windows_does_not_read_or_fallback(self):
        with self.assertRaises(ReadError) as error:
            with PreparedFixtureRead('C:\\dummy-fixture'):
                self.fail('must not open')
        self.assertEqual(str(error.exception), 'fixture_read.windows_required')


@unittest.skipUnless(sys.platform == 'win32', 'native handle proof requires Windows')
class NativeReadTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix='optic-fixed-dummy-read-')
        self.root = Path(self.temp.name)
        self.path = self.root / 'state.protected'
        self.path.write_bytes(b'PUBLIC DUMMY CIPHERTEXT SHAPE')

    def tearDown(self):
        self.temp.cleanup()

    def test_handle_read_blocks_write_and_rename_until_close(self):
        with PreparedFixtureRead(self.root) as prepared:
            self.assertEqual(prepared.read(), b'PUBLIC DUMMY CIPHERTEXT SHAPE')
            with self.assertRaises(OSError):
                self.path.write_bytes(b'CHANGED DUMMY')
            with self.assertRaises(OSError):
                self.path.rename(self.root / 'renamed.protected')
            with self.assertRaises(OSError):
                self.root.rename(self.root.with_name(self.root.name + '-renamed'))
        self.path.write_bytes(b'CHANGED DUMMY')
        with self.assertRaises(ReadError):
            prepared.read()

    def test_context_error_releases_file_and_root(self):
        with self.assertRaisesRegex(RuntimeError, 'fixed test failure'):
            with PreparedFixtureRead(self.root):
                raise RuntimeError('fixed test failure')
        self.path.rename(self.root / 'renamed.protected')

    def test_fresh_child_write_is_denied_until_parent_releases_pin(self):
        code = '''from pathlib import Path
import sys
path = Path(sys.argv[1]) / 'state.protected'
try:
    path.write_bytes(b'PUBLIC DUMMY CHILD WRITE')
except OSError:
    print('DUMMY WRITE DENIED')
else:
    print('DUMMY WRITE ALLOWED')
'''
        def attempt():
            child = subprocess.run([sys.executable, '-c', code, str(self.root)],
                                   capture_output=True, text=True, timeout=10)
            self.assertEqual(child.returncode, 0, 'fixed child did not complete')
            self.assertEqual(child.stderr, '', 'raw child diagnostics not echoed')
            return child.stdout.strip()
        with PreparedFixtureRead(self.root) as prepared:
            self.assertEqual(attempt(), 'DUMMY WRITE DENIED')
            self.assertEqual(prepared.read(), b'PUBLIC DUMMY CIPHERTEXT SHAPE')
        self.assertEqual(attempt(), 'DUMMY WRITE ALLOWED')
        self.assertEqual(self.path.read_bytes(), b'PUBLIC DUMMY CHILD WRITE')

    def test_hardlinked_file_is_rejected(self):
        alias = self.root / 'dummy-alias'
        os.link(self.path, alias)
        with self.assertRaises(ReadError) as error:
            with PreparedFixtureRead(self.root):
                self.fail('must reject alias')
        self.assertEqual(str(error.exception), 'fixture_read.link_count')
        self.path.unlink()  # rejection must release handles

    def test_empty_oversized_and_directory_target_are_rejected(self):
        for data in (b'', b'x' * (MAX_BLOB + 1)):
            self.path.write_bytes(data)
            with self.assertRaises(ReadError):
                with PreparedFixtureRead(self.root):
                    self.fail('must reject size')
        self.path.unlink()
        self.path.mkdir()
        with self.assertRaises(ReadError):
            with PreparedFixtureRead(self.root):
                self.fail('must reject directory')

    def test_missing_file_is_not_created(self):
        self.path.unlink()
        with self.assertRaises(ReadError):
            with PreparedFixtureRead(self.root):
                self.fail('must not create')
        self.assertFalse(self.path.exists())

    def test_file_and_root_symlinks_are_rejected_when_supported(self):
        target = self.root / 'dummy-target'
        target.write_bytes(b'PUBLIC DUMMY')
        self.path.unlink()
        try:
            os.symlink(target, self.path)
        except OSError:
            self.skipTest('native symlink creation unavailable; no reparse proof on this runner')
        with self.assertRaises(ReadError) as error:
            with PreparedFixtureRead(self.root):
                self.fail('must reject file reparse')
        self.assertEqual(str(error.exception), 'fixture_read.invalid_kind')
        self.path.unlink()
        self.path.write_bytes(b'PUBLIC DUMMY')
        # All links/targets remain inside the same owned test directory.
        root_link = self.root / 'dummy-root-link'
        os.symlink(self.root, root_link, target_is_directory=True)
        try:
            with self.assertRaises(ReadError) as error:
                with PreparedFixtureRead(root_link):
                    self.fail('must reject root reparse')
            self.assertEqual(str(error.exception), 'fixture_read.invalid_kind')
        finally:
            root_link.unlink()
