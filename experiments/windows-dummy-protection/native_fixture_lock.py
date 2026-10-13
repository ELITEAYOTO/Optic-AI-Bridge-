"""Exclusive native pin for one pre-existing public dummy writer.lock, no writes."""
import ctypes
import ntpath

from native_fixture_read import PreparedFixtureRead


class LockError(Exception):
    pass


class PreparedFixtureLock:
    """Root denies rename; fixed file denies all sharing until owned handle closes.

    Existing one-byte public marker only. No create, truncate, repair, delete or
    path-selected lock name. This is not yet the protected registry writer.
    """
    def __init__(self, directory):
        self.prepared = PreparedFixtureRead(directory)

    def __enter__(self):
        item = self.prepared
        # Reject re-entry before cleanup can release an already-held pin.
        if item.kernel is not None:
            raise LockError('fixture_lock.already_opened')
        try:
            item._load_native()
            item.root = item._open(item.directory, directory=True)
            item._info(item.root, directory=True)
            root_name = item._final(item.root).rstrip('\\')
            item.file = item.kernel.CreateFileW(ntpath.join(item.directory, 'writer.lock'),
                0x80000000, 0, None, 3, 0x00200000, None)
            if item.file is None or item.file == ctypes.c_void_p(-1).value:
                item.file = None
                raise LockError('fixture_lock.unavailable')
            info = item._info(item.file, directory=False)
            if ((info.size_high << 32) | info.size_low) != 1:
                raise LockError('fixture_lock.invalid_marker')
            if (item._final(item.root).rstrip('\\') != root_name
                    or item._final(item.file) != root_name + '\\writer.lock'):
                raise LockError('fixture_lock.target_changed')
            buffer = (ctypes.c_ubyte * 2)()
            count = ctypes.c_uint32()
            if (not item.kernel.ReadFile(item.file, buffer, 2, ctypes.byref(count), None)
                    or count.value != 1 or buffer[0] != ord('0')):
                raise LockError('fixture_lock.invalid_marker')
            return self
        except Exception:
            self.close()
            raise LockError('fixture_lock.unavailable') from None

    def close(self):
        try:
            self.prepared.close()
        except Exception:
            raise LockError('fixture_lock.close_failed') from None

    def __exit__(self, *args):
        self.close()
