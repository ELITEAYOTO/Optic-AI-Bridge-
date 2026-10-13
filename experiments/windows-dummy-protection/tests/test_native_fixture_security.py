import ctypes
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

from native_fixture_read import PreparedFixtureRead
from native_fixture_security import (SecurityError, acl_summary, observe_fixture_security,
                                     _sid_in_buffer, write_related, _ace_observations)


class SecurityBoundsTests(unittest.TestCase):
    def test_write_bit_summary_is_not_an_access_decision(self):
        self.assertFalse(write_related(0x80000000))  # GENERIC_READ alone
        for mask in (0x10000000, 0x40000000, 2, 4, 0x10000, 0x40000, 0x80000):
            self.assertTrue(write_related(mask))
        for mask in (True, -1, 0x100000000, 'write'):
            with self.assertRaises(SecurityError):
                write_related(mask)

    def test_outside_ace_pointer_is_rejected_before_dereference(self):
        buffer = (ctypes.c_ubyte * 32)()
        base = ctypes.addressof(buffer)
        class FakeApi:
            def GetAce(self, acl, index, output):
                output._obj.value = base - 1
                return True
        with self.assertRaises(SecurityError) as error:
            _ace_observations(FakeApi(), base, len(buffer), 1, None, None, None)
        self.assertEqual(str(error.exception), 'fixture_security.ace_bound')

    def test_unknown_ace_is_counted_without_assuming_permission_semantics(self):
        buffer = (ctypes.c_ubyte * 16)()
        base = ctypes.addressof(buffer)
        buffer[8], buffer[10] = 5, 4  # unknown object ACE, bounded header only
        class FakeApi:
            def GetAce(self, acl, index, output):
                output._obj.value = base + 8
                return True
        result = _ace_observations(FakeApi(), base, len(buffer), 1, None, None, None)
        self.assertEqual(result['unparsed_aces'], 1)
        self.assertEqual(sum(result['allow_write_aces'].values()), 0)
    def test_acl_count_and_size_bounds(self):
        for values in ((129, 8, 0), (True, 8, 0), (0, 7, 0), (0, 8, -1), (0, 65536, 1)):
            with self.assertRaises(SecurityError):
                acl_summary(*values)
        self.assertEqual(acl_summary(0, 8, 0), {'ace_count': 0})

    def test_closed_or_unprepared_input_never_loads_native_library(self):
        with patch.object(ctypes, 'WinDLL', side_effect=AssertionError('must not load'), create=True):
            for value in (None, {}, PreparedFixtureRead('C:\\dummy-fixture')):
                with self.assertRaises(SecurityError) as error:
                    observe_fixture_security(value)
                self.assertEqual(str(error.exception), 'fixture_security.open_fixture_required')

    def test_sid_pointer_bound_is_checked_before_native_interpretation(self):
        buffer = (ctypes.c_ubyte * 16)()
        base = ctypes.addressof(buffer)
        for sid in (None, base - 1, base + 9):
            with self.assertRaises(SecurityError):
                _sid_in_buffer(sid, base, len(buffer))
        buffer[1] = 15
        with self.assertRaises(SecurityError):
            _sid_in_buffer(base, base, len(buffer))


@unittest.skipUnless(sys.platform == 'win32', 'native metadata observation requires Windows')
class NativeSecurityTests(unittest.TestCase):
    def test_owner_dacl_observation_is_read_only_and_has_no_identity_values(self):
        with tempfile.TemporaryDirectory(prefix='optic-dummy-security-') as directory:
            path = Path(directory) / 'state.protected'
            path.write_bytes(b'PUBLIC DUMMY')
            with PreparedFixtureRead(directory) as prepared:
                result = observe_fixture_security(prepared)
                self.assertEqual(set(result), {'authority', 'observation_only', 'root', 'file'})
                self.assertEqual(result['authority'], 'none')
                self.assertTrue(result['observation_only'])
                for row in (result['root'], result['file']):
                    self.assertEqual(set(row), {'owner_matches_process_user', 'dacl_present', 'null_dacl', 'ace_count', 'aces', 'dacl_protected'})
                    self.assertIs(type(row['owner_matches_process_user']), bool)
                    self.assertIs(type(row['dacl_present']), bool)
                    self.assertIs(type(row['null_dacl']), bool)
                    self.assertLessEqual(row['ace_count'], 128)
                self.assertEqual(prepared.read(), b'PUBLIC DUMMY')
            path.write_bytes(b'PUBLIC DUMMY CHANGED')
