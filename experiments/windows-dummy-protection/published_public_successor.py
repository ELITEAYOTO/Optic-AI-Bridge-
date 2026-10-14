"""Closed staged public seal publication; path-based fixture, not auth state."""
import os
import stat

from create_only_successor import SuccessorError
from public_successor_reader import SealedPublicSuccessor


class _StagedSeal:
    def __init__(self, root):
        self.root = root
        self.stream = None

    def __enter__(self):
        self.stream = (self.root / 'successor.0001.seal.pending').open('x+b', buffering=0)
        try:
            info = os.fstat(self.stream.fileno())
            if not stat.S_ISREG(info.st_mode) or info.st_nlink != 1 or info.st_size != 0:
                raise SuccessorError('successor_publish.invalid_stage')
            self.identity = (info.st_dev, info.st_ino)
        except Exception:
            self.stream.close()
            raise
        return self.stream

    def _publish(self):
        source = self.root / 'successor.0001.seal.pending'
        target = self.root / 'successor.0001.seal'
        before = source.lstat()
        if (not stat.S_ISREG(before.st_mode) or before.st_nlink != 1
                or (before.st_dev, before.st_ino) != self.identity
                or getattr(before, 'st_file_attributes', 0) & 0x400):
            raise SuccessorError('successor_publish.source_changed')
        # One explicit hard-link publication, no replace/rename/copy fallback.
        # Target temporarily has two links; bounded reader refuses it until
        # the staged name is removed. Both names are fixed owned fixture files.
        os.link(source, target, follow_symlinks=False)
        left, right = source.lstat(), target.lstat()
        if ((left.st_dev, left.st_ino) != self.identity
                or (right.st_dev, right.st_ino) != self.identity
                or left.st_nlink != 2 or right.st_nlink != 2):
            raise SuccessorError('successor_publish.link_changed')
        source.unlink()

    def __exit__(self, kind, value, traceback):
        self.stream.close()  # failure prevents publication, no blind retry
        if kind is None:
            self._publish()


class PublishedPublicSuccessor(SealedPublicSuccessor):
    """One owned stage, close then no-replace link, no stage adoption/recovery.

    Requires hard-link support. Visibility with one link follows prepared
    write/flush/readback/close. No directory-fsync, hostile-writer or power-loss
    guarantee. Existing observer remains zero-authority public pair validation.
    """
    def _open_seal(self):
        return _StagedSeal(self.root)
