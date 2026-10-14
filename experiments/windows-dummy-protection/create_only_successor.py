"""One public successor-slot election, not a credential journal or native store."""
import os
from pathlib import Path
import stat

BASELINE = b'PUBLIC_SUCCESSOR_BASELINE_V1\n'
RECORDS = {'rotate-a': b'PUBLIC_SUCCESSOR_ROTATE_A\n',
           'revoke-b': b'PUBLIC_SUCCESSOR_REVOKE_B\n'}
MAX_PUBLIC = 64


class SuccessorError(Exception):
    pass


class PublicSuccessor:
    """Fixed existing baseline, one fixed CREATE_NEW slot, no overwrite/recovery.

    Path-based disposable fixture. Other same-user programs can still modify
    files. Slot existence is an election result, never an authentication ACK.
    """
    def __init__(self, directory, operation):
        if type(operation) is not str or operation not in RECORDS:
            raise SuccessorError('successor.invalid_operation')
        try:
            name = os.fspath(directory)
            if type(name) is not str or not 1 <= len(name) <= 4096 or '\0' in name:
                raise ValueError()
            self.root = Path(name)
        except (TypeError, ValueError):
            raise SuccessorError('successor.invalid_root') from None
        self.operation = operation
        self.attempted = False

    def _baseline(self):
        root_info = self.root.lstat()
        if (not stat.S_ISDIR(root_info.st_mode) or self.root.is_symlink()
                or getattr(root_info, 'st_file_attributes', 0) & 0x400):
            raise SuccessorError('successor.invalid_root')
        target = self.root / 'baseline.public'
        before = target.lstat()
        if (not stat.S_ISREG(before.st_mode) or before.st_nlink != 1
                or before.st_size != len(BASELINE)
                or getattr(before, 'st_file_attributes', 0) & 0x400):
            raise SuccessorError('successor.invalid_baseline')
        with target.open('rb', buffering=0) as stream:
            opened = os.fstat(stream.fileno())
            if (opened.st_dev, opened.st_ino) != (before.st_dev, before.st_ino):
                raise SuccessorError('successor.baseline_changed')
            value = stream.read(MAX_PUBLIC)
        if value != BASELINE:
            raise SuccessorError('successor.invalid_baseline')

    def _open_slot(self):
        # Exclusive creation is the election. No check-then-open or replacement.
        return (self.root / 'successor.0001.public').open('x+b', buffering=0)

    def claim_once(self, expected):
        if self.attempted:
            raise SuccessorError('successor.already_attempted')
        self.attempted = True
        if type(expected) is not bytes or expected != BASELINE:
            raise SuccessorError('successor.invalid_expected')
        try:
            self._baseline()
            payload = RECORDS[self.operation]
            with self._open_slot() as stream:
                if stream.write(payload) != len(payload):
                    raise SuccessorError('successor.write_failed')
                os.fsync(stream.fileno())
                stream.seek(0)
                if stream.read(MAX_PUBLIC) != payload:
                    raise SuccessorError('successor.readback_failed')
                info = os.fstat(stream.fileno())
                if not stat.S_ISREG(info.st_mode) or info.st_nlink != 1 or info.st_size != len(payload):
                    raise SuccessorError('successor.invalid_slot')
            self._baseline()
        except Exception:
            # May leave an empty/partial/complete slot: never delete or retry.
            raise SuccessorError('successor.failed') from None
        return {'ok': True, 'slot': 1, 'operation': self.operation, 'fixture_only': True}
