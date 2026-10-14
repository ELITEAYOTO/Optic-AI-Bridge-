from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

from create_only_successor import BASELINE, SuccessorError
from published_public_successor import PublishedPublicSuccessor
from native_public_pair import NativePublicPair


@unittest.skipUnless(sys.platform == 'win32', 'native Windows fixture')
class Pair(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        (self.root / 'baseline.public').write_bytes(BASELINE)
        PublishedPublicSuccessor(self.root, 'rotate-a').claim_once(BASELINE)

    def tearDown(self):
        self.temp.cleanup()

    def test_actual_other_process_writes_denied_until_owned_close(self):
        script = '''import pathlib,sys
try:
 pathlib.Path(sys.argv[1]).write_bytes(b'PUBLIC CHANGED')
except OSError:
 sys.exit(7)
sys.exit(0)
'''
        with NativePublicPair(str(self.root)) as pair:
            for leaf in ('baseline.public', 'successor.0001.public', 'successor.0001.seal'):
                result = subprocess.run([sys.executable, '-c', script, str(self.root / leaf)],
                                        timeout=10, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
                self.assertEqual(result.returncode, 7)
            self.assertEqual(pair.probe()['authority'], 'none')
            with self.assertRaises(SuccessorError):
                pair.__enter__()
            self.assertEqual(pair.probe()['authority'], 'none')
        result = subprocess.run([sys.executable, '-c', script, str(self.root / 'successor.0001.public')],
                                timeout=10, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        self.assertEqual(result.returncode, 0)
        with self.assertRaises(SuccessorError):
            pair.probe()

    def test_invalid_pair_closes_every_owned_pin_without_repair(self):
        seal = self.root / 'successor.0001.seal'
        seal.write_bytes(b'PUBLIC INVALID')
        pair = NativePublicPair(str(self.root))
        with self.assertRaises(SuccessorError):
            pair.__enter__()
        self.assertEqual(pair.handles, [])
        self.assertIsNone(pair.prepared.root)
        seal.write_bytes(b'PUBLIC STILL INVALID')
        self.assertEqual(seal.read_bytes(), b'PUBLIC STILL INVALID')
