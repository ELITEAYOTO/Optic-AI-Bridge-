import os
from pathlib import Path
import sys
import tempfile
import unittest

from native_fixture_read import PreparedFixtureRead, ReadError, fixture_directory
from dummy_dpapi import MAX_BLOB


class DirectoryTests(unittest.TestCase):
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
