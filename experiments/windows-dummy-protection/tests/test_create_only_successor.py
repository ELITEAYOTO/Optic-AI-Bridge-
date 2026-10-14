from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
import queue
import subprocess
import sys
import tempfile
import threading
import unittest
from unittest.mock import patch

from create_only_successor import BASELINE, RECORDS, PublicSuccessor, SuccessorError


class Election(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        (self.root / 'baseline.public').write_bytes(BASELINE)

    def tearDown(self):
        self.temp.cleanup()

    def test_unknown_operation_and_wrong_expected_do_not_touch_slot(self):
        for operation in ('unknown', None, {}):
            with self.assertRaises(SuccessorError):
                PublicSuccessor(self.root, operation)
        candidate = PublicSuccessor(self.root, 'rotate-a')
        with patch.object(candidate, '_baseline', side_effect=AssertionError('filesystem reached')):
            with self.assertRaises(SuccessorError):
                candidate.claim_once(b'PUBLIC WRONG')
        self.assertFalse((self.root / 'successor.0001.public').exists())

    def test_success_exhausts_single_slot_without_modifying_baseline(self):
        first = PublicSuccessor(self.root, 'rotate-a')
        self.assertEqual(first.claim_once(BASELINE),
                         {'ok': True, 'slot': 1, 'operation': 'rotate-a', 'fixture_only': True})
        with self.assertRaises(SuccessorError):
            first.claim_once(BASELINE)
        with self.assertRaises(SuccessorError):
            PublicSuccessor(self.root, 'revoke-b').claim_once(BASELINE)
        self.assertEqual((self.root / 'successor.0001.public').read_bytes(), RECORDS['rotate-a'])
        self.assertEqual((self.root / 'baseline.public').read_bytes(), BASELINE)

    def test_competitor_after_baseline_check_is_preserved(self):
        late = PublicSuccessor(self.root, 'rotate-a')
        create = late._open_slot
        def interleave():
            PublicSuccessor(self.root, 'revoke-b').claim_once(BASELINE)
            return create()
        with patch.object(late, '_open_slot', side_effect=interleave):
            with self.assertRaises(SuccessorError):
                late.claim_once(BASELINE)
        self.assertEqual((self.root / 'successor.0001.public').read_bytes(), RECORDS['revoke-b'])

    def test_actual_two_thread_election_has_one_winner(self):
        barrier = threading.Barrier(2)
        original = PublicSuccessor._open_slot
        def both_checked(candidate):
            barrier.wait(timeout=3)
            return original(candidate)
        def run(operation):
            try:
                return PublicSuccessor(self.root, operation).claim_once(BASELINE)['operation']
            except SuccessorError:
                return None
        with patch.object(PublicSuccessor, '_open_slot', both_checked):
            with ThreadPoolExecutor(max_workers=2) as executor:
                futures = [executor.submit(run, operation) for operation in RECORDS]
                results = [future.result(timeout=5) for future in futures]
        winners = [value for value in results if value is not None]
        self.assertEqual(len(winners), 1)
        self.assertEqual((self.root / 'successor.0001.public').read_bytes(), RECORDS[winners[0]])

    def test_actual_two_process_election_has_one_winner(self):
        script = '''
import sys
from create_only_successor import PublicSuccessor,BASELINE,SuccessorError
candidate=PublicSuccessor(sys.argv[1],sys.argv[2])
create=candidate._open_slot
def admitted():
    print('PUBLIC_CHECKED',flush=True)
    if sys.stdin.buffer.read(1) != b'G':
        raise SuccessorError('public.stop')
    return create()
candidate._open_slot=admitted
try:
    candidate.claim_once(BASELINE)
    print('PUBLIC_WIN:'+candidate.operation,flush=True)
except SuccessorError:
    print('PUBLIC_LOSE',flush=True)
'''
        children, readers = [], []
        ready = queue.Queue(maxsize=2)
        try:
            for index, operation in enumerate(RECORDS):
                child = subprocess.Popen([sys.executable, '-c', script, str(self.root), operation],
                    stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
                children.append(child)
                def read_ready(child=child, index=index):
                    ready.put((index, child.stdout.readline(64)))
                reader = threading.Thread(target=read_ready, daemon=True)
                readers.append(reader)
                reader.start()
            seen = set()
            for _ in range(2):
                index, line = ready.get(timeout=5)
                self.assertEqual(line.strip(), b'PUBLIC_CHECKED')
                seen.add(index)
            self.assertEqual(seen, {0, 1})
            for child in children:
                child.stdin.write(b'G')
                child.stdin.flush()
            outcomes = []
            for child in children:
                self.assertEqual(child.wait(timeout=10), 0)
                outcomes.append(child.stdout.read(128).strip())
                self.assertEqual(child.stderr.read(4096), b'')
            winners = [value for value in outcomes if value.startswith(b'PUBLIC_WIN:')]
            self.assertEqual(len(winners), 1)
            self.assertEqual(outcomes.count(b'PUBLIC_LOSE'), 1)
            operation = winners[0].split(b':', 1)[1].decode('ascii')
            self.assertIn(operation, RECORDS)
            self.assertEqual((self.root / 'successor.0001.public').read_bytes(), RECORDS[operation])
        finally:
            for child in children:
                if child.poll() is None:
                    child.kill()  # only children owned by this test
                    child.wait(timeout=10)
                for stream in (child.stdin, child.stdout, child.stderr):
                    stream.close()
            for reader in readers:
                reader.join(timeout=1)

    def test_failed_flush_has_no_success_and_slot_is_not_repaired(self):
        candidate = PublicSuccessor(self.root, 'rotate-a')
        with patch('create_only_successor.os.fsync', side_effect=OSError('PUBLIC FAILURE')):
            with self.assertRaises(SuccessorError):
                candidate.claim_once(BASELINE)
        original = (self.root / 'successor.0001.public').read_bytes()
        with self.assertRaises(SuccessorError):
            PublicSuccessor(self.root, 'revoke-b').claim_once(BASELINE)
        self.assertEqual((self.root / 'successor.0001.public').read_bytes(), original)

    def test_existing_empty_slot_is_never_promoted_or_reused(self):
        slot = self.root / 'successor.0001.public'
        slot.write_bytes(b'')
        with self.assertRaises(SuccessorError):
            PublicSuccessor(self.root, 'rotate-a').claim_once(BASELINE)
        self.assertEqual(slot.read_bytes(), b'')

    def test_partial_write_leaves_inert_slot_and_cannot_retry(self):
        candidate = PublicSuccessor(self.root, 'rotate-a')
        create = candidate._open_slot
        class Partial:
            def __enter__(self):
                self.stream = create()
                return self
            def write(self, payload):
                return self.stream.write(payload[:-1])
            def __exit__(self, *args):
                self.stream.close()
        with patch.object(candidate, '_open_slot', return_value=Partial()):
            with self.assertRaises(SuccessorError):
                candidate.claim_once(BASELINE)
        slot = self.root / 'successor.0001.public'
        self.assertEqual(slot.read_bytes(), RECORDS['rotate-a'][:-1])
        with self.assertRaises(SuccessorError):
            PublicSuccessor(self.root, 'revoke-b').claim_once(BASELINE)
        self.assertEqual(slot.read_bytes(), RECORDS['rotate-a'][:-1])

    def test_existing_file_directory_and_hardlink_are_preserved(self):
        for kind in ('file', 'directory', 'hardlink'):
            with tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                (root / 'baseline.public').write_bytes(BASELINE)
                slot = root / 'successor.0001.public'
                if kind == 'directory':
                    slot.mkdir()
                elif kind == 'hardlink':
                    source = root / 'original.public'
                    source.write_bytes(b'PUBLIC EXISTING')
                    slot.hardlink_to(source)
                else:
                    slot.write_bytes(b'PUBLIC EXISTING')
                with self.assertRaises(SuccessorError):
                    PublicSuccessor(root, 'rotate-a').claim_once(BASELINE)
                if kind == 'directory':
                    self.assertTrue(slot.is_dir())
                else:
                    self.assertEqual(slot.read_bytes(), b'PUBLIC EXISTING')

    def test_missing_or_corrupt_baseline_does_not_initialize(self):
        baseline = self.root / 'baseline.public'
        baseline.unlink()
        with self.assertRaises(SuccessorError):
            PublicSuccessor(self.root, 'rotate-a').claim_once(BASELINE)
        baseline.write_bytes(b'X' * len(BASELINE))
        with self.assertRaises(SuccessorError):
            PublicSuccessor(self.root, 'rotate-a').claim_once(BASELINE)
        self.assertFalse((self.root / 'successor.0001.public').exists())
