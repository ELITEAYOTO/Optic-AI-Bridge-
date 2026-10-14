from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

from create_only_successor import BASELINE
from published_public_successor import PublishedPublicSuccessor
from relative_public_pair import RelativePublicPair


@unittest.skipUnless(sys.platform == 'win32', 'native Windows fixture')
class RootSelection(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.parent = Path(self.temp.name)
        self.root = self.parent / 'selected'
        self.root.mkdir()
        (self.root / 'baseline.public').write_bytes(BASELINE)
        PublishedPublicSuccessor(self.root, 'rotate-a').claim_once(BASELINE)

    def tearDown(self):
        self.temp.cleanup()

    def test_negative_valid_same_name_substitution_before_root_open_is_accepted(self):
        # Deliberate negative characterization, not a claim that this is safe.
        alternate = self.parent / 'alternate'
        alternate.mkdir()
        (alternate / 'baseline.public').write_bytes(BASELINE)
        PublishedPublicSuccessor(alternate, 'revoke-b').claim_once(BASELINE)
        original_id = (self.root.stat().st_dev, self.root.stat().st_ino)
        pair = RelativePublicPair(str(self.root))
        original_open = pair.prepared._open
        swapped = []
        def swap_before_open(path, directory):
            self.assertTrue(directory)
            self.root.rename(self.parent / 'original')
            alternate.rename(self.root)
            swapped.append(True)
            return original_open(path, directory)
        with patch.object(pair.prepared, '_open', swap_before_open):
            with pair:
                self.assertEqual(pair.operation, 'revoke-b')
                self.assertEqual(pair.probe()['authority'], 'none')
        self.assertEqual(swapped, [True])
        self.assertNotEqual(original_id, (self.root.stat().st_dev, self.root.stat().st_ino))
        self.assertTrue((self.parent / 'original' / 'successor.0001.seal').exists())

    def test_held_root_refuses_ordinary_rename_then_releases_after_close(self):
        destination = self.parent / 'moved'
        with RelativePublicPair(str(self.root)) as pair:
            with self.assertRaises(OSError):
                self.root.rename(destination)
            self.assertEqual(pair.operation, 'rotate-a')
            self.assertFalse(destination.exists())
        self.root.rename(destination)
        self.assertTrue(destination.exists())
