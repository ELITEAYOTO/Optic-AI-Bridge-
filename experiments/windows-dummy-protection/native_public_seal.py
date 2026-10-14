"""Explicit Windows x64 handle-relative public seal fixture, no path unlink."""
import ctypes
import io
import os
import sys

from create_only_successor import SuccessorError
from native_fixture_read import PreparedFixtureRead, FileInfo
from native_relative_pending import UnicodeName, ObjectAttributes, IoStatus
from public_successor_reader import SealedPublicSuccessor

STAGE = 'successor.0001.seal.pending'
FINAL = 'successor.0001.seal'


class LinkPacket(ctypes.Structure):
    _fields_ = [('replace', ctypes.c_uint32), ('root', ctypes.c_void_p),
                ('length', ctypes.c_uint32), ('name', ctypes.c_wchar * 20)]


class _NativeSeal:
    def __init__(self, root):
        self.prepared = PreparedFixtureRead(str(root))
        self.stream = None
        self.descriptor = None
        self.used = False

    def __enter__(self):
        if self.used:
            raise SuccessorError('native_seal.already_attempted')
        self.used = True  # reject re-entry before cleanup can release live ownership
        item = self.prepared
        try:
            if (sys.platform != 'win32' or ctypes.sizeof(ctypes.c_void_p) != 8
                    or ctypes.sizeof(ctypes.c_wchar) != 2
                    or ctypes.sizeof(ObjectAttributes) != 48 or ctypes.sizeof(IoStatus) != 16
                    or LinkPacket.name.offset != 20 or ctypes.sizeof(LinkPacket) != 64):
                raise SuccessorError('native_seal.unsupported_abi')
            item._load_native()
            item.root = item._open(item.directory, directory=True)
            item._info(item.root, directory=True)
            self.root_name = item._final(item.root).rstrip('\\')
            self.native = ctypes.WinDLL('ntdll.dll', use_last_error=True, winmode=0x800)
            self.native.NtCreateFile.argtypes = [ctypes.POINTER(ctypes.c_void_p), ctypes.c_uint32,
                ctypes.POINTER(ObjectAttributes), ctypes.POINTER(IoStatus), ctypes.c_void_p,
                ctypes.c_uint32, ctypes.c_uint32, ctypes.c_uint32, ctypes.c_uint32,
                ctypes.c_void_p, ctypes.c_uint32]
            self.native.NtCreateFile.restype = ctypes.c_int32
            buffer = ctypes.create_unicode_buffer(STAGE)
            name = UnicodeName(len(STAGE) * 2, (len(STAGE) + 1) * 2, ctypes.cast(buffer, ctypes.c_void_p))
            attributes = ObjectAttributes(48, item.root, ctypes.pointer(name), 0x40, None, None)
            output, completion = ctypes.c_void_p(), IoStatus()
            status = self.native.NtCreateFile(ctypes.byref(output), 0x0013019F,
                ctypes.byref(attributes), ctypes.byref(completion), None, 0x80,
                0, 2, 0x00200060, None, 0)
            if status < 0:
                raise SuccessorError('native_seal.create_failed')
            if output.value is None or output.value == ctypes.c_void_p(-1).value:
                raise SuccessorError('native_seal.invalid_handle')
            item.file = output.value
            if status != 0 or completion.result.status != 0 or completion.information != 2:
                raise SuccessorError('native_seal.unexpected_completion')
            info = item._info(item.file, directory=False)
            if info.size_high or info.size_low or item._final(item.file) != self.root_name + '\\' + STAGE:
                raise SuccessorError('native_seal.invalid_stage')
            kernel = item.kernel
            kernel.GetCurrentProcess.argtypes = []
            kernel.GetCurrentProcess.restype = ctypes.c_void_p
            kernel.DuplicateHandle.argtypes = [ctypes.c_void_p, ctypes.c_void_p, ctypes.c_void_p,
                ctypes.POINTER(ctypes.c_void_p), ctypes.c_uint32, ctypes.c_int, ctypes.c_uint32]
            kernel.DuplicateHandle.restype = ctypes.c_int
            duplicate = ctypes.c_void_p()
            process = kernel.GetCurrentProcess()
            if (not kernel.DuplicateHandle(process, item.file, process, ctypes.byref(duplicate), 0, False, 2)
                    or duplicate.value is None or duplicate.value == ctypes.c_void_p(-1).value):
                raise SuccessorError('native_seal.duplicate_failed')
            import msvcrt
            try:
                self.descriptor = msvcrt.open_osfhandle(duplicate.value, os.O_RDWR | os.O_BINARY)
            except Exception:
                if not kernel.CloseHandle(duplicate):
                    raise SuccessorError('native_seal.close_failed') from None
                raise
            # FileIO is a borrowed view. This context alone owns the CRT fd,
            # including failed view construction; avoid double-close on error.
            self.stream = io.FileIO(self.descriptor, 'r+', closefd=False)
            return self.stream
        except Exception:
            try:
                self._close_descriptor()
            finally:
                item.close()
            raise SuccessorError('native_seal.entry_failed') from None

    def _close_descriptor(self):
        descriptor, self.descriptor = self.descriptor, None
        if descriptor is not None:
            os.close(descriptor)  # exactly one owned close attempt, no retry

    def _set_info(self, packet, kind):
        self.native.NtSetInformationFile.argtypes = [ctypes.c_void_p,
            ctypes.POINTER(IoStatus), ctypes.c_void_p, ctypes.c_uint32, ctypes.c_int]
        self.native.NtSetInformationFile.restype = ctypes.c_int32
        completion = IoStatus()
        status = self.native.NtSetInformationFile(self.prepared.file, ctypes.byref(completion),
            ctypes.byref(packet), ctypes.sizeof(packet), kind)
        if status != 0 or completion.result.status != 0:
            raise SuccessorError('native_seal.operation_failed')

    def _publish(self):
        item = self.prepared
        if not self.stream.closed or self.descriptor is not None:
            raise SuccessorError('native_seal.stream_open')
        info = item._info(item.file, directory=False)
        if info.size_high or info.size_low != 65:
            raise SuccessorError('native_seal.size_changed')
        if (item._final(item.root).rstrip('\\') != self.root_name
                or item._final(item.file) != self.root_name + '\\' + STAGE):
            raise SuccessorError('native_seal.identity_changed')
        self._set_info(LinkPacket(0, item.root, len(FINAL) * 2, FINAL), 11)
        info = FileInfo()
        if not item.kernel.GetFileInformationByHandle(item.file, ctypes.byref(info)) or info.links != 2:
            raise SuccessorError('native_seal.link_changed')
        # Fixed source delete disposition on the owned source handle, then close.
        # No more operations on this file handle after disposition succeeds.
        self._set_info(ctypes.c_ubyte(1), 13)

    def __exit__(self, kind, value, traceback):
        try:
            try:
                self.stream.close()  # borrowed view; original NT handle remains
            finally:
                self._close_descriptor()  # closes the owned CRT duplicate
            if kind is None:
                self._publish()
        finally:
            self.prepared.close()  # file first, then root; uncertainty is sticky


class NativePublishedPublicSuccessor(SealedPublicSuccessor):
    """Explicit native selection only; no path-based or other-platform fallback."""
    def _open_seal(self):
        return _NativeSeal(self.root)
