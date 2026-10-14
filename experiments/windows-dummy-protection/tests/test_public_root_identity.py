from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

from create_only_successor import BASELINE, SuccessorError
from published_public_successor import PublishedPublicSuccessor
from relative_public_pair import RelativePublicPair
from public_root_identity import PublicRootIdentity, ExpectedRootPublicPair, capture_public_root


class Bounds(unittest.TestCase):
    def test_invalid_expected_rejected_before_directory_or_native_io(self):
        for value in (None, (1, 2), PublicRootIdentity(True, 1), PublicRootIdentity(1, 0),
                      PublicRootIdentity(-1, 1), PublicRootIdentity(1, 2**64)):
            with self.assertRaisesRegex(SuccessorError, 'invalid_expected'):
                ExpectedRootPublicPair(object(), value)


@unittest.skipUnless(sys.platform == 'win32', 'native Windows fixture')
class Identity(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.parent = Path(self.temp.name)
        self.root = self.parent / 'selected'
        self.root.mkdir()
        (self.root / 'baseline.public').write_bytes(BASELINE)
        PublishedPublicSuccessor(self.root, 'rotate-a').claim_once(BASELINE)

    def tearDown(self):
        self.temp.cleanup()

    def test_unchanged_root_reopens_and_closed_capture_refuses(self):
        with RelativePublicPair(str(self.root)) as pair:
            expected = capture_public_root(pair)
        with self.assertRaises(SuccessorError):
            capture_public_root(pair)
        with ExpectedRootPublicPair(str(self.root), expected) as reopened:
            self.assertEqual(reopened.operation, 'rotate-a')
            self.assertEqual(capture_public_root(reopened), expected)

    def test_same_name_substitution_refused_before_any_leaf_open(self):
        with RelativePublicPair(str(self.root)) as original:
            expected = capture_public_root(original)
        alternate = self.parent / 'alternate'
        alternate.mkdir()
        (alternate / 'baseline.public').write_bytes(BASELINE)
        PublishedPublicSuccessor(alternate, 'revoke-b').claim_once(BASELINE)
        self.root.rename(self.parent / 'original')
        alternate.rename(self.root)
        pair = ExpectedRootPublicPair(str(self.root), expected)
        with patch.object(pair, '_open_public', side_effect=AssertionError('leaf')) as leaves:
            with self.assertRaises(SuccessorError):
                pair.__enter__()
            leaves.assert_not_called()
        self.assertIsNone(pair.prepared.root)
        self.assertEqual(pair.handles, [])
        (self.root / 'baseline.public').write_bytes(BASELINE)
