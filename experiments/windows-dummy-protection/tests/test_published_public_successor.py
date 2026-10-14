from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

from create_only_successor import BASELINE, SuccessorError
from public_successor_reader import SEALS, observe_public_successor
from published_public_successor import PublishedPublicSuccessor, _StagedSeal


class Published(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        (self.root / 'baseline.public').write_bytes(BASELINE)

    def tearDown(self):
        self.temp.cleanup()

    def test_reader_reserved_until_staged_seal_is_closed_and_published(self):
        original = _StagedSeal._publish
        seen = []
        def inspect(stage):
            self.assertTrue(stage.stream.closed)
            self.assertEqual((self.root / 'successor.0001.seal.pending').read_bytes(), SEALS['rotate-a'])
            seen.append(observe_public_successor(self.root)['status'])
            original(stage)
        with patch.object(_StagedSeal, '_publish', inspect):
            result = PublishedPublicSuccessor(self.root, 'rotate-a').claim_once(BASELINE)
        self.assertEqual(seen, ['reserved'])
        self.assertEqual(result['status'], 'validated')
        self.assertEqual(result['authority'], 'none')
        self.assertFalse((self.root / 'successor.0001.seal.pending').exists())

    def test_failed_entry_metadata_query_closes_owned_stage(self):
        stage = _StagedSeal(self.root)
        with patch('published_public_successor.os.fstat', side_effect=OSError('PUBLIC FAILURE')):
            with self.assertRaises(OSError):
                with stage:
                    self.fail('invalid stage admitted')
        self.assertTrue(stage.stream.closed)
        self.assertEqual((self.root / 'successor.0001.seal.pending').read_bytes(), b'')
        self.assertFalse((self.root / 'successor.0001.seal').exists())

    def test_failed_close_prevents_publication_even_with_complete_stage(self):
        original = _StagedSeal.__enter__
        class Uncertain:
            def __init__(self, stream):
                self.stream = stream
            def __getattr__(self, name):
                return getattr(self.stream, name)
            def close(self):
                self.stream.close()  # release actual descriptor for test cleanup
                raise OSError('PUBLIC CLOSE FAILURE')
        def opened(stage):
            original(stage)
            stage.stream = Uncertain(stage.stream)
            return stage.stream
        with patch.object(_StagedSeal, '__enter__', opened):
            with self.assertRaises(SuccessorError):
                PublishedPublicSuccessor(self.root, 'rotate-a').claim_once(BASELINE)
        self.assertFalse((self.root / 'successor.0001.seal').exists())
        self.assertEqual(observe_public_successor(self.root)['status'], 'reserved')

    def test_failed_stage_flush_never_publishes_visible_seal(self):
        with patch('public_successor_reader.os.fsync', side_effect=[None, OSError('PUBLIC FAILURE')]):
            with self.assertRaises(SuccessorError):
                PublishedPublicSuccessor(self.root, 'rotate-a').claim_once(BASELINE)
        self.assertFalse((self.root / 'successor.0001.seal').exists())
        self.assertEqual(observe_public_successor(self.root)['status'], 'reserved')
        self.assertEqual((self.root / 'successor.0001.seal.pending').read_bytes(), SEALS['rotate-a'])

    def test_preexisting_stage_is_preserved_without_adoption(self):
        stage = self.root / 'successor.0001.seal.pending'
        stage.write_bytes(SEALS['revoke-b'])
        with self.assertRaises(SuccessorError):
            PublishedPublicSuccessor(self.root, 'rotate-a').claim_once(BASELINE)
        self.assertEqual(stage.read_bytes(), SEALS['revoke-b'])
        self.assertFalse((self.root / 'successor.0001.seal').exists())

    def test_link_collision_preserves_competitor_without_fallback(self):
        import os
        original = os.link
        target = self.root / 'successor.0001.seal'
        def collide(source, destination, **kwargs):
            target.write_bytes(b'PUBLIC COMPETITOR')
            return original(source, destination, **kwargs)
        with patch('published_public_successor.os.link', side_effect=collide):
            with self.assertRaises(SuccessorError):
                PublishedPublicSuccessor(self.root, 'rotate-a').claim_once(BASELINE)
        self.assertEqual(target.read_bytes(), b'PUBLIC COMPETITOR')
        self.assertTrue((self.root / 'successor.0001.seal.pending').exists())
        with self.assertRaises(SuccessorError):
            observe_public_successor(self.root)

    def test_failed_stage_unlink_leaves_two_links_reader_refuses(self):
        original = Path.unlink
        def fail_stage(path, *args, **kwargs):
            if path == self.root / 'successor.0001.seal.pending':
                raise OSError('PUBLIC FAILURE')
            return original(path, *args, **kwargs)
        with patch.object(Path, 'unlink', fail_stage):
            with self.assertRaises(SuccessorError):
                PublishedPublicSuccessor(self.root, 'rotate-a').claim_once(BASELINE)
        self.assertEqual((self.root / 'successor.0001.seal').stat().st_nlink, 2)
        with self.assertRaises(SuccessorError):
            observe_public_successor(self.root)

    def test_actual_exit_at_publication_boundaries_never_invents_reply(self):
        script = '''
import os,sys
from pathlib import Path
import public_successor_reader as reader
from published_public_successor import PublishedPublicSuccessor,_StagedSeal
from create_only_successor import BASELINE
phase=sys.argv[2]
if phase=='before-publish':
    def stop(stage):
        assert stage.stream.closed
        os._exit(17)
    _StagedSeal._publish=stop
elif phase=='linked':
    original=Path.unlink
    def stop(path,*args,**kwargs):
        if path.name=='successor.0001.seal.pending':
            os._exit(17)
        return original(path,*args,**kwargs)
    Path.unlink=stop
else:
    original=reader.observe_public_successor
    def stop(root):
        result=original(root)
        if result['status']=='validated':
            os._exit(17)
        return result
    reader.observe_public_successor=stop
PublishedPublicSuccessor(sys.argv[1],'rotate-a').claim_once(BASELINE)
print('PUBLIC_UNEXPECTED_REPLY',flush=True)
'''
        for phase, expected in (('before-publish', 'reserved'), ('linked', 'refused'),
                                ('before-reply', 'validated')):
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
                if expected == 'refused':
                    self.assertEqual((root / 'successor.0001.seal').stat().st_nlink, 2)
                    with self.assertRaises(SuccessorError):
                        observe_public_successor(root)
                else:
                    result = observe_public_successor(root)
                    self.assertEqual(result['status'], expected)
                    self.assertEqual(result['authority'], 'none')
                with self.assertRaises(SuccessorError):
                    PublishedPublicSuccessor(root, 'revoke-b').claim_once(BASELINE)
