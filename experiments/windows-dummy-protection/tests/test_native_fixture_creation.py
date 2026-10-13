import ctypes
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

from native_fixture_creation import CreationError, create_private_fixture
from native_fixture_read import PreparedFixtureRead
from native_fixture_security import observe_fixture_security
from protected_registry import ProtectedStore, ProtectedRegistry
from lifecycle_core import Registry, TOKENS


class CreationInputTests(unittest.TestCase):
    def test_unopened_parent_rejected_before_native_load(self):
        with patch.object(ctypes, 'WinDLL', side_effect=AssertionError('must not load'), create=True):
            for parent in (None, {}, PreparedFixtureRead('C:\\dummy-fixture')):
                with self.assertRaises(CreationError):
                    create_private_fixture(parent)


@unittest.skipUnless(sys.platform == 'win32', 'native creation proof requires Windows')
class NativeCreationTests(unittest.TestCase):
    def test_protected_dummy_lifecycle_inside_explicit_acl_fixture(self):
        with tempfile.TemporaryDirectory(prefix='optic-dummy-parent-') as directory:
            (Path(directory) / 'state.protected').write_bytes(b'PUBLIC PARENT DUMMY')
            with PreparedFixtureRead(directory) as parent:
                root = create_private_fixture(parent)
                store = ProtectedStore(root)
                try:
                    store.write(Registry().states, initialize=True)
                finally:
                    store.close()
                state = ProtectedRegistry(root)
                initial = state.authenticate(b'Bearer ' + TOKENS['A'][0])
                self.assertTrue(state.control('rotate', 'A')['persisted'])
                state.close()
                state = ProtectedRegistry(root)
                try:
                    self.assertIsNone(state.authenticate(b'Bearer ' + TOKENS['A'][0]))
                    self.assertEqual(state.authenticate(b'Bearer ' + TOKENS['A'][1]).principal, initial.principal)
                    with PreparedFixtureRead(root) as child:
                        security = observe_fixture_security(child)
                        for row in (security['root'], security['file']):
                            self.assertEqual(row['ace_count'], 3)
                            self.assertEqual(row['aces']['allow_write_aces']['other'], 0)
                    self.assertTrue(state.control('revoke', 'A')['persisted'])
                finally:
                    state.close()
                state = ProtectedRegistry(root)
                try:
                    self.assertIsNone(state.authenticate(b'Bearer ' + TOKENS['A'][1]))
                    self.assertIsNotNone(state.authenticate(b'Bearer ' + TOKENS['B'][0]))
                finally:
                    state.close()

    def test_existing_file_child_is_not_truncated(self):
        with tempfile.TemporaryDirectory(prefix='optic-dummy-parent-') as directory:
            path = Path(directory)
            (path / 'state.protected').write_bytes(b'PUBLIC PARENT DUMMY')
            target = path / 'private-dummy-fixture'
            target.write_bytes(b'PUBLIC EXISTING FILE')
            with PreparedFixtureRead(directory) as parent:
                with self.assertRaises(CreationError):
                    create_private_fixture(parent)
            self.assertEqual(target.read_bytes(), b'PUBLIC EXISTING FILE')

    def test_creation_time_acl_and_child_inheritance(self):
        with tempfile.TemporaryDirectory(prefix='optic-dummy-parent-') as directory:
            (Path(directory) / 'state.protected').write_bytes(b'PUBLIC PARENT DUMMY')
            with PreparedFixtureRead(directory) as parent:
                root = create_private_fixture(parent)
                self.assertTrue(root.is_dir())
                (root / 'state.protected').write_bytes(b'PUBLIC CHILD DUMMY')
                with PreparedFixtureRead(root) as child:
                    result = observe_fixture_security(child)
                    self.assertTrue(result['root']['dacl_protected'])
                    for row in (result['root'], result['file']):
                        self.assertTrue(row['owner_matches_process_user'])
                        self.assertTrue(row['dacl_present'])
                        self.assertFalse(row['null_dacl'])
                        self.assertEqual(row['ace_count'], 3)
                        self.assertEqual(row['aces']['allow_write_aces']['other'], 0)
                        self.assertEqual(row['aces']['unparsed_aces'], 0)
                    self.assertEqual(child.read(), b'PUBLIC CHILD DUMMY')

    def test_existing_child_is_never_adopted_repaired_or_overwritten(self):
        with tempfile.TemporaryDirectory(prefix='optic-dummy-parent-') as directory:
            parent_path = Path(directory)
            (parent_path / 'state.protected').write_bytes(b'PUBLIC PARENT DUMMY')
            existing = parent_path / 'private-dummy-fixture'
            existing.mkdir()
            (existing / 'state.protected').write_bytes(b'PUBLIC EXISTING DUMMY')
            with PreparedFixtureRead(existing) as child:
                before = observe_fixture_security(child)
            with PreparedFixtureRead(directory) as parent:
                with self.assertRaises(CreationError) as error:
                    create_private_fixture(parent)
                self.assertEqual(str(error.exception), 'fixture_create.create_failed')
            with PreparedFixtureRead(existing) as child:
                self.assertEqual(observe_fixture_security(child), before)
                self.assertEqual(child.read(), b'PUBLIC EXISTING DUMMY')
