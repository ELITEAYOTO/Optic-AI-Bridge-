"""Create one native scratch file under an owned lock; fixed public bytes only."""
import ctypes

from native_fixture_lock import PreparedFixtureLock

PUBLIC_PENDING = b'OPTIC_PUBLIC_DUMMY_PENDING_V1\n'


class PendingError(Exception):
    pass


class PreparedFixturePending:
    """One CREATE_NEW handle, never state promotion, deletion or arbitrary payload."""
    def __init__(self, pin):
        self.pin = pin
        self.file = None
        self.kernel = None
        self.used = self.closed = self.failed = self.attempted = self.close_uncertain = False

    def _require_pin(self):
        if type(self.pin) is not PreparedFixtureLock:
            raise PendingError('fixture_pending.lock_required')
        item = self.pin.prepared
        if item.root is None or item.file is None or item.kernel is None or item.close_uncertain:
            raise PendingError('fixture_pending.lock_required')
        return item

    def __enter__(self):
        if self.used or self.closed:
            raise PendingError('fixture_pending.already_used')
        self.used = True
        item = self._require_pin()
        self.kernel = item.kernel
        try:
            self.root_name = item._final(item.root).rstrip('\\')
            self.file = self._create(item)
            if self.file is None or self.file == ctypes.c_void_p(-1).value:
                self.file = None
                raise PendingError('fixture_pending.create_failed')
            info = item._info(self.file, directory=False)
            if info.size_high or info.size_low or self.kernel is not item.kernel:
                raise PendingError('fixture_pending.identity_failed')
            self._identity(item)
            self.kernel.WriteFile.argtypes = [ctypes.c_void_p, ctypes.c_void_p, ctypes.c_uint32,
                ctypes.POINTER(ctypes.c_uint32), ctypes.c_void_p]
            self.kernel.WriteFile.restype = ctypes.c_int
            self.kernel.FlushFileBuffers.argtypes = [ctypes.c_void_p]
            self.kernel.FlushFileBuffers.restype = ctypes.c_int
            return self
        except Exception:
            self.failed = True
            self.close()
            raise PendingError('fixture_pending.create_failed') from None

    def _create(self, item):
        return self.kernel.CreateFileW(self.root_name + '\\state.protected.pending',
            0xC0000000, 0, None, 1, 0x00200000, None)

    def _identity(self, item):
        if (item.kernel is not self.kernel
                or item._final(item.root).rstrip('\\') != self.root_name
                or item._final(self.file) != self.root_name + '\\state.protected.pending'):
            raise PendingError('fixture_pending.identity_failed')

    def write_once(self):
        if self.file is None or self.closed or self.failed or self.attempted:
            raise PendingError('fixture_pending.not_writable')
        self.attempted = True
        try:
            item = self._require_pin()
            self._identity(item)
            buffer = ctypes.create_string_buffer(PUBLIC_PENDING)
            count = ctypes.c_uint32()
            if (not self.kernel.WriteFile(self.file, buffer, len(PUBLIC_PENDING), ctypes.byref(count), None)
                    or count.value != len(PUBLIC_PENDING)):
                raise PendingError('fixture_pending.write_failed')
            if not self.kernel.FlushFileBuffers(self.file):
                raise PendingError('fixture_pending.flush_failed')
            info = item._info(self.file, directory=False)
            if ((info.size_high << 32) | info.size_low) != len(PUBLIC_PENDING):
                raise PendingError('fixture_pending.size_changed')
            self._identity(item)
        except Exception:
            self.failed = True
            raise PendingError('fixture_pending.write_failed') from None

    def close(self):
        if self.close_uncertain:
            raise PendingError('fixture_pending.close_failed')
        self.closed = True
        if self.file is not None:
            handle, self.file = self.file, None
            if not self.kernel.CloseHandle(handle):
                self.close_uncertain = True
                raise PendingError('fixture_pending.close_failed')

    def __exit__(self, *args):
        self.close()
