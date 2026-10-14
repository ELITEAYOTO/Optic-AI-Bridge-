from pathlib import Path
import ctypes
import os
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

from create_only_successor import BASELINE, SuccessorError
from public_successor_reader import observe_public_successor, SEALS
from native_public_seal import NativePublishedPublicSuccessor, _NativeSeal
from native_relative_pending import IoStatus


@unittest.skipUnless(sys.platform == 'win32', 'native Windows x64 seal publication')
class Native(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        (self.root / 'baseline.public').write_bytes(BASELINE)

    def tearDown(self):
        self.temp.cleanup()

    def test_actual_native_link_and_owned_source_disposition_without_path_unlink(self):
        with patch.object(Path, 'unlink', side_effect=AssertionError('path unlink reached')):
            result = NativePublishedPublicSuccessor(self.root, 'rotate-a').claim_once(BASELINE)
        self.assertEqual(result['status'], 'validated')
        self.assertEqual(result['authority'], 'none')
        self.assertFalse((self.root / 'successor.0001.seal.pending').exists())
        self.assertEqual((self.root / 'successor.0001.seal').stat().st_nlink, 1)

    def test_reentry_refusal_does_not_release_live_file_or_descriptor(self):
        stage = _NativeSeal(self.root)
        with stage as stream:
            with self.assertRaises(SuccessorError):
                stage.__enter__()
            self.assertFalse(stream.closed)
            self.assertIsNotNone(stage.descriptor)
            self.assertIsNotNone(stage.prepared.file)
            self.assertEqual(stream.write(SEALS['rotate-a']), 65)
            os.fsync(stream.fileno())
            stream.seek(0)
            self.assertEqual(stream.read(96), SEALS['rotate-a'])
        self.assertEqual((self.root / 'successor.0001.seal').read_bytes(), SEALS['rotate-a'])

    def test_actual_root_source_and_crt_descriptor_are_non_inheritable(self):
        stage = _NativeSeal(self.root)
        with stage as stream:
            self.assertFalse(os.get_inheritable(stream.fileno()))
            kernel = stage.prepared.kernel
            kernel.GetHandleInformation.argtypes = [ctypes.c_void_p, ctypes.POINTER(ctypes.c_uint32)]
            kernel.GetHandleInformation.restype = ctypes.c_int
            for handle in (stage.prepared.root, stage.prepared.file):
                flags = ctypes.c_uint32()
                self.assertTrue(kernel.GetHandleInformation(handle, ctypes.byref(flags)))
                self.assertEqual(flags.value & 1, 0)
            self.assertEqual(stream.write(SEALS['rotate-a']), 65)
            os.fsync(stream.fileno())

    def test_shared_read_is_refused_while_original_handle_stays_open(self):
        original = _NativeSeal._set_info
        observations = []
        def inspect(stage, packet, kind):
            original(stage, packet, kind)
            if kind == 11:
                with self.assertRaises(SuccessorError):
                    observe_public_successor(self.root)
                observations.append(kind)
        with patch.object(_NativeSeal, '_set_info', inspect):
            result = NativePublishedPublicSuccessor(self.root, 'rotate-a').claim_once(BASELINE)
        self.assertEqual(observations, [11])
        self.assertEqual(result['status'], 'validated')

    def test_disposition_failure_retains_two_links_and_no_success(self):
        original = _NativeSeal._set_info
        def fail(stage, packet, kind):
            if kind == 13:
                raise OSError('PUBLIC DISPOSITION FAILURE')
            return original(stage, packet, kind)
        with patch.object(_NativeSeal, '_set_info', fail):
            with self.assertRaises(SuccessorError):
                NativePublishedPublicSuccessor(self.root, 'rotate-a').claim_once(BASELINE)
        self.assertEqual((self.root / 'successor.0001.seal').stat().st_nlink, 2)
        with self.assertRaises(SuccessorError):
            observe_public_successor(self.root)

    def test_existing_native_stage_preserved(self):
        stage = self.root / 'successor.0001.seal.pending'
        stage.write_bytes(b'PUBLIC EXISTING')
        with self.assertRaises(SuccessorError):
            NativePublishedPublicSuccessor(self.root, 'rotate-a').claim_once(BASELINE)
        self.assertEqual(stage.read_bytes(), b'PUBLIC EXISTING')
        self.assertFalse((self.root / 'successor.0001.seal').exists())

    def test_failed_file_view_construction_closes_owned_descriptor_once(self):
        with patch('native_public_seal.io.FileIO', side_effect=OSError('PUBLIC VIEW FAILURE')):
            with patch('native_public_seal.os.close', wraps=os.close) as close:
                with self.assertRaises(SuccessorError):
                    NativePublishedPublicSuccessor(self.root, 'rotate-a').claim_once(BASELINE)
                self.assertEqual(close.call_count, 1)
        self.assertEqual((self.root / 'successor.0001.seal.pending').read_bytes(), b'')
        self.assertEqual(observe_public_successor(self.root)['status'], 'reserved')

    def test_descriptor_close_uncertainty_blocks_publish_without_retry(self):
        original = _NativeSeal._close_descriptor
        closed = []
        def uncertain(stage):
            original(stage)  # actual owned descriptor released for test cleanup
            self.assertIsNone(stage.descriptor)
            closed.append(1)
            raise OSError('PUBLIC CLOSE UNCERTAINTY')
        candidate = NativePublishedPublicSuccessor(self.root, 'rotate-a')
        with patch.object(_NativeSeal, '_close_descriptor', uncertain):
            with self.assertRaises(SuccessorError):
                candidate.claim_once(BASELINE)
            with self.assertRaises(SuccessorError):
                candidate.claim_once(BASELINE)
        self.assertEqual(closed, [1])
        self.assertFalse((self.root / 'successor.0001.seal').exists())
        self.assertEqual(observe_public_successor(self.root)['status'], 'reserved')

    def test_native_link_collision_preserves_target_without_path_fallback(self):
        original = _NativeSeal._set_info
        target = self.root / 'successor.0001.seal'
        def collide(stage, packet, kind):
            if kind == 11:
                target.write_bytes(b'PUBLIC COMPETITOR')
            return original(stage, packet, kind)
        with patch.object(_NativeSeal, '_set_info', collide):
            with self.assertRaises(SuccessorError):
                NativePublishedPublicSuccessor(self.root, 'rotate-a').claim_once(BASELINE)
        self.assertEqual(target.read_bytes(), b'PUBLIC COMPETITOR')
        self.assertTrue((self.root / 'successor.0001.seal.pending').exists())

    def test_ambiguous_native_link_completion_is_refused(self):
        original = _NativeSeal._publish
        for status, completion_status in ((1, 0), (0, 1), (-1, 0)):
            with tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                (root / 'baseline.public').write_bytes(BASELINE)
                calls = []
                def rejected(stage):
                    def set_info(handle, completion, packet, size, kind):
                        calls.append((size, kind))
                        ctypes.cast(completion, ctypes.POINTER(IoStatus)).contents.result.status = completion_status
                        return status
                    with patch.object(stage.native, 'NtSetInformationFile', side_effect=set_info):
                        original(stage)
                with patch.object(_NativeSeal, '_publish', rejected):
                    with self.assertRaises(SuccessorError):
                        NativePublishedPublicSuccessor(root, 'rotate-a').claim_once(BASELINE)
                self.assertEqual(calls, [(64, 11)])
                self.assertFalse((root / 'successor.0001.seal').exists())

    def test_actual_exit_after_link_or_disposition_preserves_exact_uncertainty(self):
        script = '''
import os,sys
from native_public_seal import NativePublishedPublicSuccessor,_NativeSeal
from create_only_successor import BASELINE
original=_NativeSeal._set_info
stop_kind=11 if sys.argv[2]=='linked' else 13
def exit_after(stage,packet,kind):
    original(stage,packet,kind)
    if kind==stop_kind:
        os._exit(17)
_NativeSeal._set_info=exit_after
NativePublishedPublicSuccessor(sys.argv[1],'rotate-a').claim_once(BASELINE)
print('PUBLIC_UNEXPECTED_ACK',flush=True)
'''
        for phase in ('linked', 'disposed'):
            with tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                (root / 'baseline.public').write_bytes(BASELINE)
                child = subprocess.Popen([sys.executable, '-c', script, directory, phase],
                                         stdout=subprocess.PIPE, stderr=subprocess.PIPE)
                try:
                    self.assertEqual(child.wait(timeout=10), 17)
                    self.assertEqual(child.stdout.read(64), b'')
                    self.assertEqual(child.stderr.read(4096), b'')
                finally:
                    if child.poll() is None:
                        child.kill()
                        child.wait(timeout=10)
                    child.stdout.close()
                    child.stderr.close()
                if phase == 'linked':
                    self.assertEqual((root / 'successor.0001.seal').stat().st_nlink, 2)
                    with self.assertRaises(SuccessorError):
                        observe_public_successor(root)
                else:
                    self.assertFalse((root / 'successor.0001.seal.pending').exists())
                    result = observe_public_successor(root)
                    self.assertEqual(result['status'], 'validated')
                    self.assertEqual(result['authority'], 'none')
