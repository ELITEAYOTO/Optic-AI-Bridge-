"""Fixed public pair held through computation; Windows fixture only."""
import ctypes
import hashlib

from create_only_successor import BASELINE, RECORDS, SuccessorError
from native_fixture_read import PreparedFixtureRead
from public_successor_reader import SEALS
from public_successor_admission import PublicSuccessorAdmission


class NativePublicPair:
    def __init__(self, directory):
        self.prepared = PreparedFixtureRead(directory)
        self.handles = []  # fixed ceiling: exactly three files
        self.used = False
        self.failed_close = False
        self.operation = None

    def __enter__(self):
        if self.used:
            raise SuccessorError('public_pair.already_attempted')
        self.used = True
        item = self.prepared
        try:
            item._load_native()
            item.root = item._open(item.directory, True)
            item._info(item.root, True)
            root_name = item._final(item.root).rstrip('\\')
            values = []
            for leaf, limit in (('baseline.public', 64), ('successor.0001.public', 64), ('successor.0001.seal', 96)):
                handle = item._open(item.directory + '\\' + leaf, False)
                self.handles.append(handle)
                info = item._info(handle, False)
                size = (info.size_high << 32) | info.size_low
                if not 0 < size <= limit or item._final(handle) != root_name + '\\' + leaf:
                    raise SuccessorError('public_pair.invalid')
                buffer = (ctypes.c_ubyte * (limit + 1))()
                count = ctypes.c_uint32()
                if not item.kernel.ReadFile(handle, buffer, len(buffer), ctypes.byref(count), None) or count.value != size:
                    raise SuccessorError('public_pair.invalid')
                values.append(bytes(buffer[:count.value]))
            if item._final(item.root).rstrip('\\') != root_name or values[0] != BASELINE:
                raise SuccessorError('public_pair.invalid')
            matches = [op for op in RECORDS if values[1:] == [RECORDS[op], SEALS[op]]]
            if len(matches) != 1:
                raise SuccessorError('public_pair.invalid')
            self.operation = matches[0]
            return self
        except BaseException:
            self.close()
            raise SuccessorError('public_pair.failed') from None

    def probe(self):
        if self.operation is None or len(self.handles) != 3 or self.failed_close:
            raise SuccessorError('public_pair.closed')
        return {'public_digest': hashlib.sha256(RECORDS[self.operation]).hexdigest(),
                'fixture_only': True, 'authority': 'none'}

    def close(self):
        if self.failed_close:
            raise SuccessorError('public_pair.close_failed')
        self.operation = None
        failed = False
        while self.handles:
            handle = self.handles.pop()
            if not self.prepared.kernel.CloseHandle(handle):
                failed = True
        try:
            self.prepared.close()
        except Exception:
            failed = True
        if failed:
            self.failed_close = True
            raise SuccessorError('public_pair.close_failed')

    def __exit__(self, *args):
        self.close()


class NativePublicAdmission(PublicSuccessorAdmission):
    """Explicit native work variant; publication remains portable.

    Admission lock spans work and close. File pins span the computation only;
    final path observation occurs after close. No production effect is admitted.
    """
    def _measure_public(self):
        with NativePublicPair(str(self._directory)) as pair:
            if pair.operation != self._operation:
                raise SuccessorError('public_pair.changed')
            result = pair.probe()
        # A close failure prevents the result from reaching the coordinator.
        return result['public_digest']
