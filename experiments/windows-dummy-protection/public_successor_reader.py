"""Bounded public reservation/seal observer. Never authentication authority."""
import hashlib
import os
import stat

from create_only_successor import BASELINE, RECORDS, MAX_PUBLIC, PublicSuccessor, SuccessorError

MAX_SEAL = 96
SEALS = {operation: hashlib.sha256(BASELINE + b'\0' + record).hexdigest().encode('ascii') + b'\n'
         for operation, record in RECORDS.items()}


def _read_optional(root, leaf, limit):
    target = root / leaf
    try:
        before = target.lstat()
    except FileNotFoundError:
        return None
    if (not stat.S_ISREG(before.st_mode) or before.st_nlink != 1
            or not 0 <= before.st_size <= limit
            or getattr(before, 'st_file_attributes', 0) & 0x400):
        raise SuccessorError('successor_reader.invalid_file')
    with target.open('rb', buffering=0) as stream:
        opened = os.fstat(stream.fileno())
        if (opened.st_dev, opened.st_ino) != (before.st_dev, before.st_ino):
            raise SuccessorError('successor_reader.changed')
        value = stream.read(limit + 1)
        after = os.fstat(stream.fileno())
        if len(value) != before.st_size or after.st_size != before.st_size:
            raise SuccessorError('successor_reader.changed')
    return value


def observe_public_successor(directory):
    """Classify exactly one public slot; validated != durable/authorized/ACKed."""
    try:
        candidate = PublicSuccessor(directory, 'rotate-a')
        candidate._baseline()
        record = _read_optional(candidate.root, 'successor.0001.public', MAX_PUBLIC)
        seal = _read_optional(candidate.root, 'successor.0001.seal', MAX_SEAL)
        operation = None
        if record is None:
            if seal is not None:
                raise SuccessorError('successor_reader.orphan_seal')
            status = 'empty'
        elif seal is None:
            status = 'reserved'
        else:
            matches = [key for key in RECORDS if record == RECORDS[key] and seal == SEALS[key]]
            if len(matches) != 1:
                raise SuccessorError('successor_reader.invalid_pair')
            operation, status = matches[0], 'validated'
        candidate._baseline()
        return {'status': status, 'operation': operation, 'fixture_only': True, 'authority': 'none'}
    except Exception:
        raise SuccessorError('successor_reader.failed') from None


class SealedPublicSuccessor(PublicSuccessor):
    """Seal only after own claim's write/readback/close; no repair or retries.

    A valid seal may be observable before its writer's flush/close/ACK. Reader
    output is public pair validation, not a durable commit protocol.
    """
    def _open_seal(self):
        return (self.root / 'successor.0001.seal').open('x+b', buffering=0)

    def claim_once(self, expected):
        if self.attempted:
            raise SuccessorError('successor.already_attempted')
        if type(expected) is not bytes or expected != BASELINE:
            return super().claim_once(expected)  # fixed rejection before I/O
        try:
            if observe_public_successor(self.root)['status'] != 'empty':
                raise SuccessorError('successor_reader.not_empty')
        except Exception:
            self.attempted = True
            raise SuccessorError('successor_reader.failed') from None
        super().claim_once(expected)
        try:
            seal = SEALS[self.operation]
            with self._open_seal() as stream:
                if stream.write(seal) != len(seal):
                    raise SuccessorError('successor_reader.seal_write_failed')
                os.fsync(stream.fileno())
                stream.seek(0)
                if stream.read(MAX_SEAL) != seal:
                    raise SuccessorError('successor_reader.seal_readback_failed')
                info = os.fstat(stream.fileno())
                if not stat.S_ISREG(info.st_mode) or info.st_nlink != 1 or info.st_size != len(seal):
                    raise SuccessorError('successor_reader.seal_changed')
            observed = observe_public_successor(self.root)
            if observed['status'] != 'validated' or observed['operation'] != self.operation:
                raise SuccessorError('successor_reader.seal_changed')
        except Exception:
            raise SuccessorError('successor_reader.failed') from None
        return observed
