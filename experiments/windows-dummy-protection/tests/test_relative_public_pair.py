from pathlib import Path
import ctypes
import sys
import tempfile
import unittest
from unittest.mock import patch

from create_only_successor import BASELINE, SuccessorError
from published_public_successor import PublishedPublicSuccessor
from relative_public_pair import RelativePublicPair
from native_relative_pending import IoStatus


@unittest.skipUnless(sys.platform == 'win32', 'native Windows x64 fixture')
class Relative(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        (self.root / 'baseline.public').write_bytes(BASELINE)
        PublishedPublicSuccessor(self.root, 'rotate-a').claim_once(BASELINE)

    def tearDown(self):
        self.temp.cleanup()

    def test_actual_leaf_open_uses_held_root_despite_mutated_directory(self):
        pair = RelativePublicPair(str(self.root))
        original = pair._open_public
        def open_relative(leaf):
            pair.prepared.directory = str(self.root / 'absent-decoy')
            return original(leaf)
        with patch.object(pair, '_open_public', open_relative):
            with pair:
                self.assertEqual(pair.probe()['authority'], 'none')
                with self.assertRaises(OSError):
                    (self.root / 'successor.0001.public').write_bytes(b'PUBLIC CHANGED')
        self.assertEqual(pair.handles, [])

    def test_missing_leaf_never_creates_and_releases_prior_handles(self):
        seal = self.root / 'successor.0001.seal'
        seal.unlink()
        pair = RelativePublicPair(str(self.root))
        with self.assertRaises(SuccessorError):
            pair.__enter__()
        self.assertFalse(seal.exists())
        self.assertEqual(pair.handles, [])
        self.assertIsNone(pair.prepared.root)
        (self.root / 'baseline.public').write_bytes(BASELINE)

    def test_unknown_leaf_rejected_before_native_loading(self):
        pair = RelativePublicPair(str(self.root))
        with self.assertRaisesRegex(SuccessorError, 'invalid_leaf'):
            pair._open_public('other')

    def test_success_owned_ambiguous_completion_closes_without_fallback(self):
        original_loader = ctypes.WinDLL
        for status, io_status, information in ((1, 0, 1), (0, 1, 1), (0, 0, 2)):
            pair = RelativePublicPair(str(self.root))
            calls = []
            def fake_open(output, access, attributes, completion, *args):
                calls.append(True)
                owned = pair.prepared._open(str(self.root / 'baseline.public'), False)
                ctypes.cast(output, ctypes.POINTER(ctypes.c_void_p)).contents.value = owned
                io = ctypes.cast(completion, ctypes.POINTER(IoStatus)).contents
                io.result.status, io.information = io_status, information
                return status
            class Native:
                NtCreateFile = staticmethod(fake_open)
            def loader(name, **kwargs):
                return Native() if name == 'ntdll.dll' else original_loader(name, **kwargs)
            with patch('relative_public_pair.ctypes.WinDLL', loader):
                with self.assertRaises(SuccessorError):
                    pair.__enter__()
            self.assertEqual(calls, [True])
            self.assertEqual(pair.handles, [])
            self.assertIsNone(pair.prepared.root)
            (self.root / 'baseline.public').write_bytes(BASELINE)

    def test_negative_completion_never_claims_unowned_output(self):
        pair = RelativePublicPair(str(self.root))
        def fake_open(output, *args):
            ctypes.cast(output, ctypes.POINTER(ctypes.c_void_p)).contents.value = 123
            return -1
        class Native:
            NtCreateFile = staticmethod(fake_open)
        pair.prepared.root, pair.prepared.kernel = 456, object()
        with patch('relative_public_pair.ctypes.WinDLL', return_value=Native()):
            with self.assertRaisesRegex(SuccessorError, 'open_failed'):
                pair._open_public('baseline.public')
        self.assertEqual(pair.handles, [])
        # Fabricated values are never closed or passed to an OS operation.
        pair.prepared.root, pair.prepared.kernel = None, None
