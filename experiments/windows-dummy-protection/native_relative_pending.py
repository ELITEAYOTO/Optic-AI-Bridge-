"""Explicit NT root-relative public scratch candidate; no path-based fallback."""
import ctypes
import sys

from native_fixture_pending import PreparedFixturePending, PendingError


class UnicodeName(ctypes.Structure):
    _fields_ = [('length', ctypes.c_uint16), ('maximum_length', ctypes.c_uint16),
                ('buffer', ctypes.c_void_p)]


class ObjectAttributes(ctypes.Structure):
    _fields_ = [('length', ctypes.c_uint32), ('root', ctypes.c_void_p),
                ('name', ctypes.POINTER(UnicodeName)), ('attributes', ctypes.c_uint32),
                ('security', ctypes.c_void_p), ('quality', ctypes.c_void_p)]


class StatusUnion(ctypes.Union):
    _fields_ = [('status', ctypes.c_int32), ('pointer', ctypes.c_void_p)]


class IoStatus(ctypes.Structure):
    _fields_ = [('result', StatusUnion), ('information', ctypes.c_size_t)]


class RootRelativeFixturePending(PreparedFixturePending):
    """Same public write/identity protocol, explicit NtCreateFile RootDirectory.

    Only fixed leaf state.protected.pending. No arbitrary object attributes,
    handle, disposition, access, privileges or create fallback exposed.
    """
    def _create(self, item):
        if (sys.platform != 'win32' or ctypes.sizeof(ctypes.c_void_p) != 8
                or ctypes.sizeof(ctypes.c_wchar) != 2
                or ctypes.sizeof(UnicodeName) != 16
                or ctypes.sizeof(ObjectAttributes) != 48 or ctypes.sizeof(IoStatus) != 16):
            raise PendingError('relative_pending.unsupported_abi')
        try:
            native = ctypes.WinDLL('ntdll.dll', use_last_error=True, winmode=0x800)
        except OSError:
            raise PendingError('relative_pending.native_unavailable') from None
        native.NtCreateFile.argtypes = [ctypes.POINTER(ctypes.c_void_p), ctypes.c_uint32,
            ctypes.POINTER(ObjectAttributes), ctypes.POINTER(IoStatus), ctypes.POINTER(ctypes.c_int64),
            ctypes.c_uint32, ctypes.c_uint32, ctypes.c_uint32, ctypes.c_uint32,
            ctypes.c_void_p, ctypes.c_uint32]
        native.NtCreateFile.restype = ctypes.c_int32
        leaf = 'state.protected.pending'
        buffer = ctypes.create_unicode_buffer(leaf)
        name = UnicodeName(len(leaf) * 2, (len(leaf) + 1) * 2, ctypes.cast(buffer, ctypes.c_void_p))
        attributes = ObjectAttributes(ctypes.sizeof(ObjectAttributes), item.root,
            ctypes.pointer(name), 0x40, None, None)  # case insensitive, non-inheritable
        output = ctypes.c_void_p()
        completion = IoStatus()
        status = native.NtCreateFile(ctypes.byref(output), 0x0012019F,
            ctypes.byref(attributes), ctypes.byref(completion), None, 0x80,
            0, 2, 0x00200060, None, 0)
        # FILE_CREATE2; synchronous nonalert/non-directory/open-reparse; no EA.
        # Only a successful returned handle is owned and passed to base cleanup.
        if status < 0:
            raise PendingError('relative_pending.create_failed')
        if output.value is None or output.value == ctypes.c_void_p(-1).value:
            raise PendingError('relative_pending.invalid_handle')
        self.file = output.value
        if status != 0 or completion.result.status != 0 or completion.information != 2:
            raise PendingError('relative_pending.unexpected_completion')
        return output.value
