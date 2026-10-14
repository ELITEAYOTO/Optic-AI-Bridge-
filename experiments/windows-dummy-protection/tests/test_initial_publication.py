import ctypes
from pathlib import Path
import sys
import subprocess
import tempfile
import unittest
from unittest.mock import patch

from native_fixture_read import PreparedFixtureRead
from native_fixture_lock import PreparedFixtureLock
from native_fixture_creation import CreationError
from native_fixture_pending import PendingError, PUBLIC_PENDING
from native_initial_publication import publish_fresh_public_fixture, _InitialPending, RenamePacket
from native_relative_pending import IoStatus


class Bounds(unittest.TestCase):
    def test_unopened_parent_does_not_create(self):
        with self.assertRaises(CreationError):
            publish_fresh_public_fixture(None)


@unittest.skipUnless(sys.platform == 'win32', 'native Windows initial publication')
class Native(unittest.TestCase):
    def _exit_child(self, directory, after_publish):
        script = '''
import os,sys
from pathlib import Path
from native_fixture_read import PreparedFixtureRead
from native_fixture_creation import create_private_fixture
from native_fixture_lock import PreparedFixtureLock
from native_initial_publication import publish_fresh_public_fixture,_InitialPending
with PreparedFixtureRead(sys.argv[1]) as parent:
    if sys.argv[2] == 'after':
        publish_fresh_public_fixture(parent)
        print('PUBLIC_ACK',flush=True)
    else:
        root=create_private_fixture(parent)
        (root/'writer.lock').write_bytes(b'0')
        with PreparedFixtureLock(root) as pin:
            with _InitialPending(pin) as pending:
                pending.write_once()
                print('PUBLIC_PENDING_ONLY',flush=True)
                os._exit(17)
os._exit(17)
'''
        child = subprocess.Popen([sys.executable, '-c', script, directory,
                                  'after' if after_publish else 'before'],
                                 stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        try:
            self.assertEqual(child.wait(timeout=10), 17)
            self.assertEqual(child.stdout.read(64).strip(),
                             b'PUBLIC_ACK' if after_publish else b'PUBLIC_PENDING_ONLY')
            self.assertEqual(child.stderr.read(4096), b'')
        finally:
            if child.poll() is None:
                child.kill()  # only the child owned by this test
                child.wait(timeout=10)
            child.stdout.close()
            child.stderr.close()

    def test_abrupt_exit_before_publish_does_not_promote_pending_on_reopen(self):
        with tempfile.TemporaryDirectory() as directory:
            (Path(directory) / 'state.protected').write_bytes(b'PUBLIC PARENT')
            self._exit_child(directory, False)
            root = Path(directory) / 'private-dummy-fixture'
            self.assertFalse((root / 'state.protected').exists())
            self.assertEqual((root / 'state.protected.pending').read_bytes(), PUBLIC_PENDING)
            with PreparedFixtureRead(directory) as parent:
                with self.assertRaises(CreationError):
                    publish_fresh_public_fixture(parent)
            self.assertFalse((root / 'state.protected').exists())
            with PreparedFixtureLock(root):
                pass  # actual kernel pin released despite os._exit

    def test_abrupt_exit_after_ack_retains_marker_and_refuses_reinitialization(self):
        with tempfile.TemporaryDirectory() as directory:
            (Path(directory) / 'state.protected').write_bytes(b'PUBLIC PARENT')
            self._exit_child(directory, True)
            root = Path(directory) / 'private-dummy-fixture'
            with PreparedFixtureRead(root) as published:
                self.assertEqual(published.read(), PUBLIC_PENDING)
            self.assertFalse((root / 'state.protected.pending').exists())
            with PreparedFixtureRead(directory) as parent:
                with self.assertRaises(CreationError):
                    publish_fresh_public_fixture(parent)

    def test_wrong_same_length_public_bytes_are_denied_before_rename(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'writer.lock').write_bytes(b'0')
            with PreparedFixtureLock(root) as pin:
                with _InitialPending(pin) as pending:
                    write = pending.kernel.WriteFile
                    def corrupt(handle, buffer, size, count, overlapped):
                        public_wrong = ctypes.create_string_buffer(b'X' * len(PUBLIC_PENDING))
                        return write(handle, public_wrong, size, count, overlapped)
                    with patch.object(pending.kernel, 'WriteFile', side_effect=corrupt):
                        pending.write_once()
                    with patch.object(pending, '_rename', side_effect=AssertionError('rename reached')) as rename:
                        with self.assertRaises(PendingError):
                            pending.publish_once()
                        rename.assert_not_called()
            self.assertFalse((root / 'state.protected').exists())
            self.assertEqual((root / 'state.protected.pending').read_bytes(), b'X' * len(PUBLIC_PENDING))

    def test_ambiguous_native_completion_never_acknowledges_or_retries(self):
        for status, io_status in ((1, 0), (0, 1), (-1, 0)):
            with tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                (root / 'writer.lock').write_bytes(b'0')
                with PreparedFixtureLock(root) as pin:
                    with _InitialPending(pin) as pending:
                        pending.write_once()
                        calls = []
                        class SetInfo:
                            def __call__(self, handle, completion, packet, size, kind):
                                calls.append((handle, size, kind))
                                ctypes.cast(completion, ctypes.POINTER(IoStatus)).contents.result.status = io_status
                                return status
                        class Library:
                            NtSetInformationFile = SetInfo()
                        with patch.object(ctypes, 'WinDLL', return_value=Library()):
                            with self.assertRaises(PendingError):
                                pending.publish_once()
                            with self.assertRaises(PendingError):
                                pending.publish_once()
                        self.assertEqual(calls, [(pending.file, 56, 10)])
                self.assertFalse((root / 'state.protected').exists())

    def test_post_rename_validation_failure_keeps_uncertainty_without_ack(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'writer.lock').write_bytes(b'0')
            with PreparedFixtureLock(root) as pin:
                with _InitialPending(pin) as pending:
                    pending.write_once()
                    verify = pending._verify_bytes
                    calls = []
                    def fail_after_rename(item):
                        calls.append(1)
                        if len(calls) == 2:
                            raise OSError('PUBLIC POST RENAME FAILURE')
                        verify(item)
                    with patch.object(pending, '_verify_bytes', side_effect=fail_after_rename):
                        with self.assertRaises(PendingError):
                            pending.publish_once()
                        with self.assertRaises(PendingError):
                            pending.publish_once()
                    self.assertTrue(pending.failed)
            self.assertEqual((root / 'state.protected').read_bytes(), PUBLIC_PENDING)
            self.assertFalse((root / 'state.protected.pending').exists())

    def test_close_failure_after_publication_prevents_wrapper_success(self):
        with tempfile.TemporaryDirectory() as directory:
            (Path(directory) / 'state.protected').write_bytes(b'PUBLIC PARENT')
            original_close = _InitialPending.close
            def uncertain_close(pending):
                original_close(pending)  # release actual owned OS handle in test
                pending.close_uncertain = True
                raise PendingError('fixture_pending.close_failed')
            with PreparedFixtureRead(directory) as parent:
                with patch.object(_InitialPending, 'close', uncertain_close):
                    with self.assertRaises(PendingError):
                        publish_fresh_public_fixture(parent)
            target = Path(directory) / 'private-dummy-fixture' / 'state.protected'
            self.assertEqual(target.read_bytes(), PUBLIC_PENDING)

    def test_fresh_publication_and_missing_old_state_never_reset(self):
        with tempfile.TemporaryDirectory() as directory:
            (Path(directory) / 'state.protected').write_bytes(b'PUBLIC PARENT')
            with PreparedFixtureRead(directory) as parent:
                root = publish_fresh_public_fixture(parent)
                self.assertEqual((root / 'state.protected').read_bytes(), PUBLIC_PENDING)
                self.assertFalse((root / 'state.protected.pending').exists())
                (root / 'state.protected').unlink()
                with self.assertRaises(CreationError):
                    publish_fresh_public_fixture(parent)
                self.assertFalse((root / 'state.protected').exists())
                self.assertEqual((root / 'writer.lock').read_bytes(), b'0')

    def test_existing_target_collision_preserves_file_directory_and_hardlink(self):
        for kind in ('file', 'directory', 'hardlink'):
            with tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                (root / 'writer.lock').write_bytes(b'0')
                target = root / 'state.protected'
                if kind == 'directory':
                    target.mkdir()
                elif kind == 'hardlink':
                    original = root / 'original'
                    original.write_bytes(b'PUBLIC EXISTING')
                    target.hardlink_to(original)
                else:
                    target.write_bytes(b'PUBLIC EXISTING')
                with PreparedFixtureLock(root) as pin:
                    with _InitialPending(pin) as pending:
                        pending.write_once()
                        with self.assertRaises(PendingError):
                            pending.publish_once()
                        with self.assertRaises(PendingError):
                            pending.publish_once()
                self.assertEqual((root / 'state.protected.pending').read_bytes(), PUBLIC_PENDING)
                if kind == 'directory':
                    self.assertTrue(target.is_dir())
                else:
                    self.assertEqual(target.read_bytes(), b'PUBLIC EXISTING')

    def test_native_failure_is_single_attempt_with_fixed_no_replace_packet(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'writer.lock').write_bytes(b'0')
            with PreparedFixtureLock(root) as pin:
                with _InitialPending(pin) as pending:
                    pending.write_once()
                    calls = []
                    def reject(packet):
                        calls.append((packet.flags, packet.root, packet.name))
                        raise OSError('PUBLIC SIMULATED FAILURE')
                    with patch.object(pending, '_rename', side_effect=reject):
                        with self.assertRaises(PendingError):
                            pending.publish_once()
                        with self.assertRaises(PendingError):
                            pending.publish_once()
                    self.assertEqual(calls, [(0, pin.prepared.root, 'state.protected')])
            self.assertFalse((root / 'state.protected').exists())

    def test_success_cannot_repeat_or_write_again(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'writer.lock').write_bytes(b'0')
            with PreparedFixtureLock(root) as pin:
                with _InitialPending(pin) as pending:
                    pending.write_once()
                    pending.publish_once()
                    with self.assertRaises(PendingError):
                        pending.publish_once()
                    with self.assertRaises(PendingError):
                        pending.write_once()
            self.assertEqual((root / 'state.protected').read_bytes(), PUBLIC_PENDING)
