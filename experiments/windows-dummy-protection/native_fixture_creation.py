"""Creation-time DACL experiment for ONE new public-dummy child directory.

Never repairs/adopts existing state. No ACL mutation of existing user files.
"""
import ctypes
from pathlib import Path
import sys

from native_fixture_read import PreparedFixtureRead
from native_fixture_security import TokenUser, _sid_in_buffer


class CreationError(Exception):
    def __init__(self, code):
        super().__init__(code)
        self.code = code


class Descriptor(ctypes.Structure):
    _fields_ = [('revision', ctypes.c_ubyte), ('reserved', ctypes.c_ubyte),
                ('control', ctypes.c_uint16), ('owner', ctypes.c_void_p),
                ('group', ctypes.c_void_p), ('sacl', ctypes.c_void_p), ('dacl', ctypes.c_void_p)]


class SecurityAttributes(ctypes.Structure):
    _fields_ = [('size', ctypes.c_uint32), ('descriptor', ctypes.c_void_p), ('inherit', ctypes.c_int)]


def create_private_fixture(prepared):
    if (not isinstance(prepared, PreparedFixtureRead) or prepared.root is None
            or prepared.file is None or prepared.kernel is None or prepared.close_uncertain):
        raise CreationError('fixture_create.open_parent_required')
    if sys.platform != 'win32':
        raise CreationError('fixture_create.windows_required')
    try:
        api = ctypes.WinDLL('advapi32.dll', use_last_error=True, winmode=0x800)
    except OSError:
        raise CreationError('fixture_create.native_unavailable') from None
    pointer = ctypes.c_void_p
    pp = ctypes.POINTER(pointer)
    kernel = prepared.kernel
    kernel.GetCurrentProcess.argtypes = []
    kernel.GetCurrentProcess.restype = pointer
    kernel.CreateDirectoryW.argtypes = [ctypes.c_wchar_p, ctypes.POINTER(SecurityAttributes)]
    kernel.CreateDirectoryW.restype = ctypes.c_int
    api.OpenProcessToken.argtypes = [pointer, ctypes.c_uint32, pp]
    api.OpenProcessToken.restype = ctypes.c_int
    api.GetTokenInformation.argtypes = [pointer, ctypes.c_int, pointer, ctypes.c_uint32,
                                        ctypes.POINTER(ctypes.c_uint32)]
    api.GetTokenInformation.restype = ctypes.c_int
    api.IsValidSid.argtypes = [pointer]
    api.IsValidSid.restype = ctypes.c_int
    api.CreateWellKnownSid.argtypes = [ctypes.c_int, pointer, pointer, ctypes.POINTER(ctypes.c_uint32)]
    api.CreateWellKnownSid.restype = ctypes.c_int
    api.InitializeAcl.argtypes = [pointer, ctypes.c_uint32, ctypes.c_uint32]
    api.InitializeAcl.restype = ctypes.c_int
    api.AddAccessAllowedAceEx.argtypes = [pointer, ctypes.c_uint32, ctypes.c_uint32, ctypes.c_uint32, pointer]
    api.AddAccessAllowedAceEx.restype = ctypes.c_int
    api.InitializeSecurityDescriptor.argtypes = [pointer, ctypes.c_uint32]
    api.InitializeSecurityDescriptor.restype = ctypes.c_int
    api.SetSecurityDescriptorOwner.argtypes = [pointer, pointer, ctypes.c_int]
    api.SetSecurityDescriptorOwner.restype = ctypes.c_int
    api.SetSecurityDescriptorDacl.argtypes = [pointer, ctypes.c_int, pointer, ctypes.c_int]
    api.SetSecurityDescriptorDacl.restype = ctypes.c_int
    api.SetSecurityDescriptorControl.argtypes = [pointer, ctypes.c_uint16, ctypes.c_uint16]
    api.SetSecurityDescriptorControl.restype = ctypes.c_int
    token = pointer()
    try:
        if not api.OpenProcessToken(kernel.GetCurrentProcess(), 8, ctypes.byref(token)) or not token.value:
            raise CreationError('fixture_create.token_failed')
        needed = ctypes.c_uint32()
        api.GetTokenInformation(token, 1, None, 0, ctypes.byref(needed))
        if not ctypes.sizeof(TokenUser) <= needed.value <= 4096:
            raise CreationError('fixture_create.token_bound')
        buffer = (ctypes.c_ubyte * needed.value)()
        if not api.GetTokenInformation(token, 1, buffer, len(buffer), ctypes.byref(needed)):
            raise CreationError('fixture_create.token_failed')
        if not ctypes.sizeof(TokenUser) <= needed.value <= len(buffer):
            raise CreationError('fixture_create.token_bound')
        user = TokenUser.from_buffer(buffer)
        _sid_in_buffer(user.sid, ctypes.addressof(buffer), needed.value)
        if not api.IsValidSid(user.sid):
            raise CreationError('fixture_create.sid_invalid')
        known = []
        for kind in (22, 26):
            sid = (ctypes.c_ubyte * 68)()
            size = ctypes.c_uint32(len(sid))
            if not api.CreateWellKnownSid(kind, None, sid, ctypes.byref(size)) or not 8 <= size.value <= len(sid):
                raise CreationError('fixture_create.sid_failed')
            _sid_in_buffer(ctypes.addressof(sid), ctypes.addressof(sid), size.value)
            if not api.IsValidSid(sid):
                raise CreationError('fixture_create.sid_invalid')
            known.append(sid)
        acl = (ctypes.c_ubyte * 512)()
        descriptor = Descriptor()
        if not api.InitializeAcl(acl, len(acl), 2):
            raise CreationError('fixture_create.descriptor_failed')
        # Three fixed trustees only: process user, LocalSystem, Administrators.
        # Object/container inherit, FILE_ALL_ACCESS. No caller-supplied SID/mask.
        for sid in (user.sid, known[0], known[1]):
            if not api.AddAccessAllowedAceEx(acl, 2, 3, 0x1F01FF, sid):
                raise CreationError('fixture_create.descriptor_failed')
        if (not api.InitializeSecurityDescriptor(ctypes.byref(descriptor), 1)
                or not api.SetSecurityDescriptorOwner(ctypes.byref(descriptor), user.sid, False)
                or not api.SetSecurityDescriptorDacl(ctypes.byref(descriptor), True, acl, False)
                or not api.SetSecurityDescriptorControl(ctypes.byref(descriptor), 0x1000, 0x1000)):
            raise CreationError('fixture_create.descriptor_failed')
        attributes = SecurityAttributes(ctypes.sizeof(SecurityAttributes), ctypes.addressof(descriptor), False)
        parent_name = prepared._final(prepared.root).rstrip('\\')
        # CreateDirectory refuses an existing child; no SetSecurityInfo repair,
        # overwrite, recursive cleanup or fallback to inherited/default ACL.
        if not kernel.CreateDirectoryW(parent_name + '\\private-dummy-fixture', ctypes.byref(attributes)):
            raise CreationError('fixture_create.create_failed')
        if prepared._final(prepared.root).rstrip('\\') != parent_name:
            raise CreationError('fixture_create.parent_changed')
        return Path(prepared.directory) / 'private-dummy-fixture'
    finally:
        if token.value and not kernel.CloseHandle(token):
            raise CreationError('fixture_create.token_close_failed')
