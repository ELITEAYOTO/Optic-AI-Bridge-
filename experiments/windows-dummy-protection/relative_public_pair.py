"""Explicit held-root NT public read variant; never create or fall back."""
import ctypes
import sys

from create_only_successor import SuccessorError
from native_public_pair import NativePublicPair
from native_relative_pending import UnicodeName, ObjectAttributes, IoStatus


class RelativePublicPair(NativePublicPair):
    def _open_public(self, leaf):
        if leaf not in ('baseline.public', 'successor.0001.public', 'successor.0001.seal'):
            raise SuccessorError('relative_pair.invalid_leaf')
        if (sys.platform != 'win32' or ctypes.sizeof(ctypes.c_void_p) != 8
                or ctypes.sizeof(ctypes.c_wchar) != 2 or ctypes.sizeof(UnicodeName) != 16
                or ctypes.sizeof(ObjectAttributes) != 48 or ctypes.sizeof(IoStatus) != 16):
            raise SuccessorError('relative_pair.unsupported_abi')
        item = self.prepared
        if item.root is None or item.kernel is None:
            raise SuccessorError('relative_pair.no_root')
        native = ctypes.WinDLL('ntdll.dll', use_last_error=True, winmode=0x800)
        native.NtCreateFile.argtypes = [ctypes.POINTER(ctypes.c_void_p), ctypes.c_uint32,
            ctypes.POINTER(ObjectAttributes), ctypes.POINTER(IoStatus), ctypes.c_void_p,
            ctypes.c_uint32, ctypes.c_uint32, ctypes.c_uint32, ctypes.c_uint32,
            ctypes.c_void_p, ctypes.c_uint32]
        native.NtCreateFile.restype = ctypes.c_int32
        buffer = ctypes.create_unicode_buffer(leaf)
        name = UnicodeName(len(leaf) * 2, (len(leaf) + 1) * 2, ctypes.cast(buffer, ctypes.c_void_p))
        attributes = ObjectAttributes(48, item.root, ctypes.pointer(name), 0x40, None, None)
        output, completion = ctypes.c_void_p(), IoStatus()
        # FILE_GENERIC_READ, share read, FILE_OPEN, synchronous/non-directory/
        # open reparse itself. No write, delete, create, truncate or fallback.
        status = native.NtCreateFile(ctypes.byref(output), 0x00120089,
            ctypes.byref(attributes), ctypes.byref(completion), None, 0,
            1, 1, 0x00200060, None, 0)
        if status < 0:
            raise SuccessorError('relative_pair.open_failed')
        if output.value is None or output.value == ctypes.c_void_p(-1).value:
            raise SuccessorError('relative_pair.invalid_handle')
        if status != 0 or completion.result.status != 0 or completion.information != 1:
            self.handles.append(output.value)  # successful returned ownership only
            raise SuccessorError('relative_pair.unexpected_completion')
        return output.value
