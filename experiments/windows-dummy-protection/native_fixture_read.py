"""Native observation/read proof for fixed state.protected in a dummy directory.

No file selector, write primitive, ACL claim, remote endpoint or plaintext decode.
"""
import ctypes
import ntpath
import os
import re
import sys

from dummy_dpapi import MAX_BLOB


class ReadError(Exception):
    def __init__(self, code):
        super().__init__(code)
        self.code = code


class FileTime(ctypes.Structure):
    _fields_ = [('low', ctypes.c_uint32), ('high', ctypes.c_uint32)]


class FileInfo(ctypes.Structure):
    _fields_ = [('attributes', ctypes.c_uint32), ('created', FileTime),
                ('accessed', FileTime), ('written', FileTime),
                ('volume', ctypes.c_uint32), ('size_high', ctypes.c_uint32),
                ('size_low', ctypes.c_uint32), ('links', ctypes.c_uint32),
                ('index_high', ctypes.c_uint32), ('index_low', ctypes.c_uint32)]


def fixture_directory(directory):
    try:
        path = os.fspath(directory)
    except TypeError:
        raise ReadError('fixture_read.invalid_directory') from None
    if (not isinstance(path, str) or not 3 < len(path) < 4096 or '\0' in path
            or not re.fullmatch(r'[A-Za-z]:', ntpath.splitdrive(path)[0])
            or not ntpath.isabs(path) or ntpath.normpath(path) != path.rstrip('\\')):
        raise ReadError('fixture_read.invalid_directory')
    return path


class PreparedFixtureRead:
    """Context-owned pinned root/file; consumes only the observed file handle."""
    def __init__(self, directory):
        self.directory = fixture_directory(directory)
        self.root = self.file = None
        self.kernel = None
        self.close_uncertain = False

    def _load_native(self):
        if self.kernel is not None:
            raise ReadError('fixture_read.already_opened')
        if sys.platform != 'win32':
            raise ReadError('fixture_read.windows_required')
        try:
            kernel = ctypes.WinDLL('kernel32.dll', use_last_error=True, winmode=0x800)
        except OSError:
            raise ReadError('fixture_read.native_unavailable') from None
        self.kernel = kernel
        handle = ctypes.c_void_p
        kernel.CreateFileW.argtypes = [ctypes.c_wchar_p, ctypes.c_uint32, ctypes.c_uint32,
                                      handle, ctypes.c_uint32, ctypes.c_uint32, handle]
        kernel.CreateFileW.restype = handle
        kernel.GetFileInformationByHandle.argtypes = [handle, ctypes.POINTER(FileInfo)]
        kernel.GetFileInformationByHandle.restype = ctypes.c_int
        kernel.GetFileType.argtypes = [handle]
        kernel.GetFileType.restype = ctypes.c_uint32
        kernel.GetFinalPathNameByHandleW.argtypes = [handle, ctypes.c_wchar_p, ctypes.c_uint32, ctypes.c_uint32]
        kernel.GetFinalPathNameByHandleW.restype = ctypes.c_uint32
        kernel.ReadFile.argtypes = [handle, handle, ctypes.c_uint32, ctypes.POINTER(ctypes.c_uint32), handle]
        kernel.ReadFile.restype = ctypes.c_int
        kernel.CloseHandle.argtypes = [handle]
        kernel.CloseHandle.restype = ctypes.c_int

    def __enter__(self):
        self._load_native()
        try:
            self.root = self._open(self.directory, directory=True)
            self._info(self.root, directory=True)
            root_name = self._final(self.root).rstrip('\\')
            self.file = self._open(ntpath.join(self.directory, 'state.protected'), directory=False)
            info = self._info(self.file, directory=False)
            if (self._final(self.root).rstrip('\\') != root_name
                    or self._final(self.file) != root_name + '\\state.protected'):
                raise ReadError('fixture_read.target_mismatch')
            self.size = (info.size_high << 32) | info.size_low
            if not 0 < self.size <= MAX_BLOB:
                raise ReadError('fixture_read.size_bound')
            return self
        except Exception:
            self.close()
            raise

    def _open(self, path, directory):
        # Root: allow read/write sharing, deny delete/rename. File: read only
        # sharing, deny concurrent write/delete. OPEN_EXISTING, non-inheritable.
        flags = 0x00200000 | (0x02000000 if directory else 0)  # open reparse itself
        result = self.kernel.CreateFileW(path, 0x80000000, 3 if directory else 1,
                                         None, 3, flags, None)
        if result is None or result == ctypes.c_void_p(-1).value:
            raise ReadError('fixture_read.open_failed')
        return result

    def _info(self, handle, directory):
        info = FileInfo()
        if self.kernel.GetFileType(handle) != 1 or not self.kernel.GetFileInformationByHandle(handle, ctypes.byref(info)):
            raise ReadError('fixture_read.info_failed')
        if info.attributes & 0x400 or bool(info.attributes & 0x10) != directory:
            raise ReadError('fixture_read.invalid_kind')
        if not directory and info.links != 1:
            raise ReadError('fixture_read.link_count')
        return info

    def _final(self, handle):
        buffer = ctypes.create_unicode_buffer(4096)
        length = self.kernel.GetFinalPathNameByHandleW(handle, buffer, len(buffer), 0)
        if not 0 < length < len(buffer):
            raise ReadError('fixture_read.path_bound')
        return buffer.value

    def read(self):
        if self.file is None:
            raise ReadError('fixture_read.closed')
        buffer = (ctypes.c_ubyte * (MAX_BLOB + 1))()
        count = ctypes.c_uint32()
        if not self.kernel.ReadFile(self.file, buffer, len(buffer), ctypes.byref(count), None):
            raise ReadError('fixture_read.read_failed')
        if count.value != self.size:
            raise ReadError('fixture_read.read_changed')
        return bytes(buffer[:count.value])

    def close(self):
        if self.close_uncertain:
            raise ReadError('fixture_read.close_failed')
        failed = False
        for name in ('file', 'root'):
            handle = getattr(self, name)
            if handle is not None:
                setattr(self, name, None)
                if not self.kernel.CloseHandle(handle):
                    failed = True
        if failed:
            self.close_uncertain = True
            raise ReadError('fixture_read.close_failed')

    def __exit__(self, *args):
        self.close()
