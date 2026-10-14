from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
import tempfile
import threading
import unittest
from unittest.mock import patch

from create_only_successor import BASELINE, RECORDS, SuccessorError
from public_successor_reader import observe_public_successor
from published_public_successor import PublishedPublicSuccessor
from public_successor_admission import PublicSuccessorAdmission


class Admission(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        (self.root / 'baseline.public').write_bytes(BASELINE)

    def tearDown(self):
        self.temp.cleanup()

    def test_confirmed_snapshot_is_detached_and_zero_authority(self):
        gate = PublicSuccessorAdmission(self.root)
        self.assertEqual(gate.read()['status'], 'empty')
        result = gate.publish_once('rotate-a', BASELINE)
        self.assertEqual(result['status'], 'confirmed')
        self.assertEqual(result['authority'], 'none')
        result['operation'] = 'revoke-b'
        with self.assertRaisesRegex(SuccessorError, 'unavailable'):
            gate.publish_once('revoke-b', BASELINE)
        self.assertEqual(gate.read()['operation'], 'rotate-a')

    def test_pending_publication_refuses_read_and_second_writer_without_queue(self):
        gate = PublicSuccessorAdmission(self.root)
        entered, release = threading.Event(), threading.Event()
        original = PublishedPublicSuccessor.claim_once
        def hold(writer, expected):
            entered.set()
            if not release.wait(5):
                raise RuntimeError('test timeout')
            return original(writer, expected)
        with patch.object(PublishedPublicSuccessor, 'claim_once', hold), ThreadPoolExecutor(max_workers=1) as pool:
            future = pool.submit(gate.publish_once, 'rotate-a', BASELINE)
            try:
                self.assertTrue(entered.wait(3))
                for call in (gate.read, lambda: gate.publish_once('revoke-b', BASELINE)):
                    with self.assertRaisesRegex(SuccessorError, 'busy'):
                        call()
                self.assertFalse(future.cancel())  # running ownership remains
                self.assertFalse((self.root / 'successor.0001.public').exists())
            finally:
                release.set()
            self.assertEqual(future.result(timeout=5)['status'], 'confirmed')
        self.assertEqual(gate.read()['operation'], 'rotate-a')

    def test_failure_after_valid_pair_never_restores_admission(self):
        gate = PublicSuccessorAdmission(self.root)
        original = PublishedPublicSuccessor.claim_once
        def lost_completion(writer, expected):
            original(writer, expected)
            raise RuntimeError('public completion lost')
        with patch.object(PublishedPublicSuccessor, 'claim_once', lost_completion):
            with self.assertRaises(SuccessorError):
                gate.publish_once('rotate-a', BASELINE)
        self.assertEqual(observe_public_successor(self.root)['status'], 'validated')
        for call in (gate.read, lambda: gate.publish_once('revoke-b', BASELINE)):
            with self.assertRaises(SuccessorError):
                call()
        self.assertEqual((self.root / 'successor.0001.public').read_bytes(), RECORDS['rotate-a'])

    def test_restart_never_adopts_confirmed_or_reserved_pair(self):
        PublishedPublicSuccessor(self.root, 'rotate-a').claim_once(BASELINE)
        with self.assertRaises(SuccessorError):
            PublicSuccessorAdmission(self.root).read()
        (self.root / 'successor.0001.seal').unlink()
        with self.assertRaises(SuccessorError):
            PublicSuccessorAdmission(self.root).read()

    def test_invalid_input_preserves_empty_admission_without_io(self):
        gate = PublicSuccessorAdmission(self.root)
        with patch('public_successor_admission.observe_public_successor', side_effect=AssertionError('I/O')):
            for operation, expected in (([], BASELINE), ('other', BASELINE), ('rotate-a', b'wrong')):
                with self.assertRaisesRegex(SuccessorError, 'invalid_input'):
                    gate.publish_once(operation, expected)
        self.assertEqual(gate.read()['status'], 'empty')

    def test_changed_confirmed_pair_blocks_stickily_without_repair(self):
        gate = PublicSuccessorAdmission(self.root)
        gate.publish_once('rotate-a', BASELINE)
        record = self.root / 'successor.0001.public'
        record.write_bytes(b'PUBLIC CHANGED')
        with self.assertRaises(SuccessorError):
            gate.read()
        record.write_bytes(RECORDS['rotate-a'])
        with self.assertRaises(SuccessorError):
            gate.read()

    def test_interrupted_writer_blocks_even_before_any_record(self):
        gate = PublicSuccessorAdmission(self.root)
        with patch.object(PublishedPublicSuccessor, 'claim_once', side_effect=KeyboardInterrupt):
            with self.assertRaises(SuccessorError):
                gate.publish_once('rotate-a', BASELINE)
        self.assertEqual(observe_public_successor(self.root)['status'], 'empty')
        with self.assertRaises(SuccessorError):
            gate.read()
