from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
import hashlib
import tempfile
import threading
import unittest
from unittest.mock import patch

from create_only_successor import BASELINE, RECORDS, SuccessorError
from public_successor_admission import PublicSuccessorAdmission


class Work(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        (self.root / 'baseline.public').write_bytes(BASELINE)
        self.gate = PublicSuccessorAdmission(self.root)

    def tearDown(self):
        self.temp.cleanup()

    def test_unconfirmed_work_never_runs(self):
        with patch.object(self.gate, '_measure_public', side_effect=AssertionError('work')):
            with self.assertRaises(SuccessorError):
                self.gate.probe_confirmed()
        self.assertEqual(self.gate.read()['status'], 'empty')

    def test_exact_public_work_has_finite_attempt_budget(self):
        self.gate.publish_once('rotate-a', BASELINE)
        for attempt in range(1, 33):
            result = self.gate.probe_confirmed()
            self.assertEqual(result['attempt'], attempt)
            self.assertEqual(result['public_digest'], hashlib.sha256(RECORDS['rotate-a']).hexdigest())
            self.assertEqual(result['authority'], 'none')
        with patch.object(self.gate, '_measure_public', side_effect=AssertionError('work')):
            with self.assertRaises(SuccessorError):
                self.gate.probe_confirmed()
        self.assertEqual(self.gate.read()['status'], 'confirmed')

    def test_work_keeps_ownership_and_busy_close_is_not_acknowledged(self):
        self.gate.publish_once('rotate-a', BASELINE)
        entered, release = threading.Event(), threading.Event()
        original = self.gate._measure_public
        def hold():
            entered.set()
            if not release.wait(5):
                raise RuntimeError('test timeout')
            return original()
        with patch.object(self.gate, '_measure_public', hold), ThreadPoolExecutor(max_workers=1) as pool:
            future = pool.submit(self.gate.probe_confirmed)
            try:
                self.assertTrue(entered.wait(3))
                for call in (self.gate.read, self.gate.probe_confirmed, self.gate.close,
                             lambda: self.gate.publish_once('revoke-b', BASELINE)):
                    with self.assertRaisesRegex(SuccessorError, 'busy'):
                        call()
                self.assertFalse(future.cancel())
            finally:
                release.set()
            self.assertEqual(future.result(timeout=5)['attempt'], 1)
        self.assertTrue(self.gate.close()['closed'])
        with self.assertRaises(SuccessorError):
            self.gate.probe_confirmed()

    def test_changed_before_work_never_executes_and_blocks(self):
        self.gate.publish_once('rotate-a', BASELINE)
        (self.root / 'successor.0001.public').write_bytes(b'PUBLIC CHANGED')
        with patch.object(self.gate, '_measure_public', side_effect=AssertionError('work')) as work:
            with self.assertRaises(SuccessorError):
                self.gate.probe_confirmed()
            work.assert_not_called()

    def test_external_change_during_work_suppresses_result_without_rollback_claim(self):
        self.gate.publish_once('rotate-a', BASELINE)
        performed = []
        def changed():
            performed.append(True)
            (self.root / 'successor.0001.public').write_bytes(b'PUBLIC CHANGED')
            return 'public result'
        with patch.object(self.gate, '_measure_public', changed):
            with self.assertRaises(SuccessorError):
                self.gate.probe_confirmed()
        self.assertEqual(performed, [True])  # post-check did not undo work
        with self.assertRaises(SuccessorError):
            self.gate.read()

    def test_interrupted_work_releases_lock_but_never_reopens(self):
        self.gate.publish_once('rotate-a', BASELINE)
        with patch.object(self.gate, '_measure_public', side_effect=KeyboardInterrupt):
            with self.assertRaises(SuccessorError):
                self.gate.probe_confirmed()
        with self.assertRaisesRegex(SuccessorError, 'blocked'):
            self.gate.read()
        self.assertTrue(self.gate.close()['closed'])
