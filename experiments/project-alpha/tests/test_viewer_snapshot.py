import tempfile
from pathlib import Path
import unittest

from viewer_snapshot import FILES, write_viewer_ready


class ViewerSnapshot(unittest.TestCase):
    def test_ready_is_create_only_complete_and_bounded(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / 'project' / 'src'
            source.mkdir(parents=True)
            for name in FILES:
                (source / name).write_text('public', encoding='utf-8')
            self.assertTrue(write_viewer_ready(root)['viewer_ready'])
            with self.assertRaises(FileExistsError):
                write_viewer_ready(root)

    def test_missing_oversize_and_binary_never_publish_ready(self):
        for value in (None, b'x' * 8193, b'\xff'):
            with self.subTest(value_type=type(value).__name__), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                source = root / 'project' / 'src'
                source.mkdir(parents=True)
                for name in FILES:
                    (source / name).write_bytes(b'public')
                target = source / FILES[0]
                if value is None:
                    target.unlink()
                else:
                    target.write_bytes(value)
                with self.assertRaises((ValueError, UnicodeError)):
                    write_viewer_ready(root)
                self.assertFalse((root / 'viewer-ready.json').exists())
