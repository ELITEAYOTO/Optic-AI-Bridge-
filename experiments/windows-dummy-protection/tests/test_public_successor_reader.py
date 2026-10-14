from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

from create_only_successor import BASELINE, RECORDS, PublicSuccessor, SuccessorError
from public_successor_reader import SEALS, observe_public_successor, SealedPublicSuccessor


class Reader(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        (self.root / 'baseline.public').write_bytes(BASELINE)

    def tearDown(self):
        self.temp.cleanup()

    def test_absent_empty_partial_and_full_unsealed_remain_without_authority(self):
        self.assertEqual(observe_public_successor(self.root)['status'], 'empty')
        for record in (b'', RECORDS['rotate-a'][:-1], RECORDS['rotate-a']):
            (self.root / 'successor.0001.public').write_bytes(record)
            observed = observe_public_successor(self.root)
            self.assertEqual(observed['status'], 'reserved')
            self.assertIsNone(observed['operation'])
            self.assertEqual(observed['authority'], 'none')

    def test_each_sealed_pair_validates_without_changing_files(self):
        for operation in RECORDS:
            with tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                (root / 'baseline.public').write_bytes(BASELINE)
                observed = SealedPublicSuccessor(root, operation).claim_once(BASELINE)
                before = [(root / name).read_bytes() for name in
                          ('baseline.public', 'successor.0001.public', 'successor.0001.seal')]
                self.assertEqual(observed['status'], 'validated')
                self.assertEqual(observed['operation'], operation)
                self.assertEqual(observe_public_successor(root), observed)
                self.assertEqual([(root / name).read_bytes() for name in
                    ('baseline.public', 'successor.0001.public', 'successor.0001.seal')], before)

    def test_orphan_partial_mismatched_and_unknown_seals_fail_without_repair(self):
        record = self.root / 'successor.0001.public'
        seal = self.root / 'successor.0001.seal'
        seal.write_bytes(SEALS['rotate-a'])
        with self.assertRaises(SuccessorError):
            observe_public_successor(self.root)
        record.write_bytes(RECORDS['rotate-a'])
        for value in (b'', SEALS['rotate-a'][:-1], SEALS['revoke-b'], b'PUBLIC UNKNOWN'):
            seal.write_bytes(value)
            with self.assertRaises(SuccessorError):
                observe_public_successor(self.root)
            self.assertEqual(seal.read_bytes(), value)

    def test_failed_seal_flush_can_leave_validated_pair_without_success(self):
        candidate = SealedPublicSuccessor(self.root, 'rotate-a')
        # Same os module: first flush completes ordinary claim; second is seal.
        with patch('public_successor_reader.os.fsync', side_effect=[None, OSError('PUBLIC FAILURE')]):
            with self.assertRaises(SuccessorError):
                candidate.claim_once(BASELINE)
        self.assertEqual(observe_public_successor(self.root)['status'], 'validated')
        with self.assertRaises(SuccessorError):
            candidate.claim_once(BASELINE)

    def test_actual_exit_before_seal_or_reply_does_not_invent_ack(self):
        script = '''
import os,sys
import public_successor_reader as reader
from create_only_successor import BASELINE
candidate=reader.SealedPublicSuccessor(sys.argv[1],'rotate-a')
if sys.argv[2]=='before-seal':
    candidate._open_seal=lambda:os._exit(17)
else:
    observe=reader.observe_public_successor
    def before_reply(root):
        result=observe(root)
        if result['status']=='validated':
            os._exit(17)
        return result
    reader.observe_public_successor=before_reply
candidate.claim_once(BASELINE)
print('PUBLIC_UNEXPECTED_ACK',flush=True)
'''
        for phase, expected in (('before-seal', 'reserved'), ('before-reply', 'validated')):
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
                observed = observe_public_successor(root)
                self.assertEqual(observed['status'], expected)
                self.assertEqual(observed['authority'], 'none')
                with self.assertRaises(SuccessorError):
                    SealedPublicSuccessor(root, 'revoke-b').claim_once(BASELINE)

    def test_hardlinked_and_changed_sealed_records_are_refused(self):
        candidate = SealedPublicSuccessor(self.root, 'rotate-a')
        candidate.claim_once(BASELINE)
        record = self.root / 'successor.0001.public'
        extra = self.root / 'extra.public'
        extra.hardlink_to(record)
        with self.assertRaises(SuccessorError):
            observe_public_successor(self.root)
        extra.unlink()
        record.write_bytes(RECORDS['revoke-b'])
        with self.assertRaises(SuccessorError):
            observe_public_successor(self.root)
        self.assertEqual(record.read_bytes(), RECORDS['revoke-b'])

    def test_existing_seal_preserved_and_no_unsealed_adoption(self):
        (self.root / 'successor.0001.seal').write_bytes(b'PUBLIC EXISTING')
        with self.assertRaises(SuccessorError):
            SealedPublicSuccessor(self.root, 'rotate-a').claim_once(BASELINE)
        self.assertEqual((self.root / 'successor.0001.seal').read_bytes(), b'PUBLIC EXISTING')
        self.assertFalse((self.root / 'successor.0001.public').exists())
        with self.assertRaises(SuccessorError):
            observe_public_successor(self.root)

    def test_wrong_expected_is_rejected_before_reader_io(self):
        candidate = SealedPublicSuccessor(self.root, 'rotate-a')
        with patch('public_successor_reader.observe_public_successor', side_effect=AssertionError('reader reached')):
            with self.assertRaises(SuccessorError):
                candidate.claim_once(b'PUBLIC WRONG')
        self.assertFalse((self.root / 'successor.0001.public').exists())

    def test_oversized_record_seal_and_missing_baseline_fail_closed(self):
        record = self.root / 'successor.0001.public'
        seal = self.root / 'successor.0001.seal'
        record.write_bytes(b'X' * 65)
        with self.assertRaises(SuccessorError):
            observe_public_successor(self.root)
        record.write_bytes(RECORDS['rotate-a'])
        seal.write_bytes(b'X' * 97)
        with self.assertRaises(SuccessorError):
            observe_public_successor(self.root)
        (self.root / 'baseline.public').unlink()
        with self.assertRaises(SuccessorError):
            observe_public_successor(self.root)
