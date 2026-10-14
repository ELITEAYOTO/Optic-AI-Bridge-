"""Fresh public-marker fixture publication only; never registry initialization."""
import ctypes

from native_fixture_creation import create_private_fixture
from native_fixture_lock import PreparedFixtureLock
from native_fixture_pending import PendingError, PUBLIC_PENDING
from native_relative_pending import RootRelativeFixturePending, IoStatus


class RenamePacket(ctypes.Structure):
    _fields_ = [('flags', ctypes.c_uint32), ('root', ctypes.c_void_p),
                ('length', ctypes.c_uint32), ('name', ctypes.c_wchar * 16)]


class _InitialPending(RootRelativeFixturePending):
    """Internal fixed rename candidate; DELETE applies only to this scratch."""
    def _desired_access(self):
        return super()._desired_access() | 0x00010000

    def _verify_bytes(self, item):
        self.kernel.SetFilePointerEx.argtypes = [ctypes.c_void_p, ctypes.c_int64,
                                                ctypes.c_void_p, ctypes.c_uint32]
        self.kernel.SetFilePointerEx.restype = ctypes.c_int
        if not self.kernel.SetFilePointerEx(self.file, 0, None, 0):
            raise PendingError('initial_publish.seek_failed')
        buffer = (ctypes.c_ubyte * (len(PUBLIC_PENDING) + 1))()
        count = ctypes.c_uint32()
        if (not self.kernel.ReadFile(self.file, buffer, len(buffer), ctypes.byref(count), None)
                or count.value != len(PUBLIC_PENDING)
                or bytes(buffer[:count.value]) != PUBLIC_PENDING):
            raise PendingError('initial_publish.read_failed')
        info = item._info(self.file, directory=False)
        if ((info.size_high << 32) | info.size_low) != len(PUBLIC_PENDING):
            raise PendingError('initial_publish.size_changed')

    def publish_once(self):
        if self.file is None or self.closed or self.failed or not self.attempted:
            raise PendingError('initial_publish.not_ready')
        if getattr(self, 'publish_attempted', False):
            raise PendingError('initial_publish.already_attempted')
        self.publish_attempted = True
        try:
            item = self._require_pin()
            self._identity(item)
            self._verify_bytes(item)
            if (RenamePacket.root.offset != 8 or RenamePacket.name.offset != 20
                    or ctypes.sizeof(RenamePacket) != 56):
                raise PendingError('initial_publish.unsupported_abi')
            leaf = 'state.protected'
            packet = RenamePacket(0, item.root, len(leaf) * 2, leaf)
            self._rename(packet)
            if (item._final(item.root).rstrip('\\') != self.root_name
                    or item._final(self.file) != self.root_name + '\\state.protected'):
                raise PendingError('initial_publish.target_changed')
            self._verify_bytes(item)
        except Exception:
            self.failed = True
            raise PendingError('initial_publish.failed') from None

    def _rename(self, packet):
        native = ctypes.WinDLL('ntdll.dll', use_last_error=True, winmode=0x800)
        native.NtSetInformationFile.argtypes = [ctypes.c_void_p,
            ctypes.POINTER(IoStatus), ctypes.c_void_p, ctypes.c_uint32, ctypes.c_int]
        native.NtSetInformationFile.restype = ctypes.c_int32
        completion = IoStatus()
        # FileRenameInformation10; zero replace/flags; no fallback API.
        status = native.NtSetInformationFile(self.file, ctypes.byref(completion),
            ctypes.byref(packet), ctypes.sizeof(packet), 10)
        if status != 0 or completion.result.status != 0:
            raise PendingError('initial_publish.rename_failed')


def publish_fresh_public_fixture(parent):
    """Requires a live prepared dummy parent; refuses every existing child.

    Creates the fixed private child in this operation; missing state in an old
    child is never initialization authority. This path-based root creation
    still does not prove hostile ancestor exclusion or power-loss durability.
    """
    root = create_private_fixture(parent)
    with (root / 'writer.lock').open('xb') as marker:
        marker.write(b'0')
    with PreparedFixtureLock(root) as pin:
        with _InitialPending(pin) as pending:
            pending.write_once()
            pending.publish_once()
    # Success only after both contexts close without uncertainty.
    return root
