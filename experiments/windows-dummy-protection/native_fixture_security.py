"""Read-only owner/DACL characterization of pinned public dummy fixture handles.

NOT an authorization policy, effective-access evaluator or ACL repair tool.
"""
import ctypes
import sys

from native_fixture_read import PreparedFixtureRead


class SecurityError(Exception):
    def __init__(self, code):
        super().__init__(code)
        self.code = code


class TokenUser(ctypes.Structure):
    _fields_ = [('sid', ctypes.c_void_p), ('attributes', ctypes.c_uint32)]


class AclSize(ctypes.Structure):
    _fields_ = [('count', ctypes.c_uint32), ('used', ctypes.c_uint32), ('free', ctypes.c_uint32)]


class AclHeader(ctypes.Structure):
    _fields_ = [('revision', ctypes.c_ubyte), ('reserved', ctypes.c_ubyte),
                ('size', ctypes.c_uint16), ('count', ctypes.c_uint16), ('reserved2', ctypes.c_uint16)]


class AceHeader(ctypes.Structure):
    _fields_ = [('kind', ctypes.c_ubyte), ('flags', ctypes.c_ubyte), ('size', ctypes.c_uint16)]


WRITE_RELATED_MASK = 0x500D0156  # generic ALL/WRITE, delete/DACL/owner and file modification bits


def write_related(mask):
    if type(mask) is not int or not 0 <= mask <= 0xffffffff:
        raise SecurityError('fixture_security.mask_invalid')
    return bool(mask & WRITE_RELATED_MASK)


def _ace_observations(api, dacl, size, count, user, system, administrators):
    # Counts are NOT an effective-access or safety decision. DENY ordering,
    # groups/conditions and inheritance semantics are intentionally not evaluated.
    result = {'allow_write_aces': {'process_user': 0, 'system': 0, 'administrators': 0, 'other': 0},
              'deny_aces': 0, 'inherit_only_aces': 0, 'unparsed_aces': 0}
    for index in range(count):
        ace = ctypes.c_void_p()
        if not api.GetAce(dacl, index, ctypes.byref(ace)):
            raise SecurityError('fixture_security.ace_query_failed')
        if not ace.value or ace.value % 4 or not dacl + 8 <= ace.value or ace.value + 4 > dacl + size:
            raise SecurityError('fixture_security.ace_bound')
        header = AceHeader.from_address(ace.value)
        if header.size < 4 or ace.value + header.size > dacl + size:
            raise SecurityError('fixture_security.ace_bound')
        if header.flags & 8:
            result['inherit_only_aces'] += 1
        if header.kind not in (0, 1):
            result['unparsed_aces'] += 1
            continue
        if header.size < 16:
            raise SecurityError('fixture_security.ace_bound')
        sid = ace.value + 8
        _sid_in_buffer(sid, ace.value, header.size)
        if not api.IsValidSid(sid):
            raise SecurityError('fixture_security.sid_invalid')
        if header.kind == 1:
            result['deny_aces'] += 1
            continue
        mask = ctypes.c_uint32.from_address(ace.value + 4).value
        if not write_related(mask):
            continue
        category = 'other'
        for name, known in (('process_user', user), ('system', system), ('administrators', administrators)):
            if api.EqualSid(sid, known):
                category = name
                break
        result['allow_write_aces'][category] += 1
    return result


def acl_summary(count, used, free):
    if (any(type(value) is not int for value in (count, used, free)) or not 0 <= count <= 128
            or not 8 <= used <= 65536 or free < 0 or used + free > 65536):
        raise SecurityError('fixture_security.acl_bound')
    return {'ace_count': count}


def _sid_in_buffer(sid, base, size):
    if not sid or sid % 4 or not base <= sid or sid + 8 > base + size:
        raise SecurityError('fixture_security.sid_bound')
    subauthorities = ctypes.c_ubyte.from_address(sid + 1).value
    if subauthorities > 15 or sid + 8 + 4 * subauthorities > base + size:
        raise SecurityError('fixture_security.sid_bound')


def observe_fixture_security(prepared):
    if not isinstance(prepared, PreparedFixtureRead) or prepared.root is None or prepared.file is None:
        raise SecurityError('fixture_security.open_fixture_required')
    if sys.platform != 'win32':
        raise SecurityError('fixture_security.windows_required')
    try:
        api = ctypes.WinDLL('advapi32.dll', use_last_error=True, winmode=0x800)
    except OSError:
        raise SecurityError('fixture_security.native_unavailable') from None
    pointer = ctypes.c_void_p
    pp = ctypes.POINTER(pointer)
    kernel = prepared.kernel
    kernel.GetCurrentProcess.argtypes = []
    kernel.GetCurrentProcess.restype = pointer
    kernel.LocalFree.argtypes = [pointer]
    kernel.LocalFree.restype = pointer
    api.OpenProcessToken.argtypes = [pointer, ctypes.c_uint32, pp]
    api.OpenProcessToken.restype = ctypes.c_int
    api.GetTokenInformation.argtypes = [pointer, ctypes.c_int, pointer, ctypes.c_uint32,
                                        ctypes.POINTER(ctypes.c_uint32)]
    api.GetTokenInformation.restype = ctypes.c_int
    api.GetSecurityInfo.argtypes = [pointer, ctypes.c_int, ctypes.c_uint32, pp, pp, pp, pp, pp]
    api.GetSecurityInfo.restype = ctypes.c_uint32
    api.GetSecurityDescriptorLength.argtypes = [pointer]
    api.GetSecurityDescriptorLength.restype = ctypes.c_uint32
    api.GetSecurityDescriptorDacl.argtypes = [pointer, ctypes.POINTER(ctypes.c_int), pp,
                                            ctypes.POINTER(ctypes.c_int)]
    api.GetSecurityDescriptorDacl.restype = ctypes.c_int
    api.GetAclInformation.argtypes = [pointer, pointer, ctypes.c_uint32, ctypes.c_int]
    api.GetAclInformation.restype = ctypes.c_int
    api.IsValidSid.argtypes = [pointer]
    api.IsValidSid.restype = ctypes.c_int
    api.EqualSid.argtypes = [pointer, pointer]
    api.EqualSid.restype = ctypes.c_int
    api.CreateWellKnownSid.argtypes = [ctypes.c_int, pointer, pointer, ctypes.POINTER(ctypes.c_uint32)]
    api.CreateWellKnownSid.restype = ctypes.c_int
    api.GetAce.argtypes = [pointer, ctypes.c_uint32, pp]
    api.GetAce.restype = ctypes.c_int
    token = pointer()
    try:
        if not api.OpenProcessToken(kernel.GetCurrentProcess(), 8, ctypes.byref(token)) or not token.value:
            raise SecurityError('fixture_security.token_query_failed')
        needed = ctypes.c_uint32()
        api.GetTokenInformation(token, 1, None, 0, ctypes.byref(needed))
        if not ctypes.sizeof(TokenUser) <= needed.value <= 4096:
            raise SecurityError('fixture_security.token_bound')
        buffer = (ctypes.c_ubyte * needed.value)()
        if not api.GetTokenInformation(token, 1, buffer, len(buffer), ctypes.byref(needed)):
            raise SecurityError('fixture_security.token_query_failed')
        if not ctypes.sizeof(TokenUser) <= needed.value <= len(buffer):
            raise SecurityError('fixture_security.token_bound')
        user = TokenUser.from_buffer(buffer)
        _sid_in_buffer(user.sid, ctypes.addressof(buffer), needed.value)
        if not api.IsValidSid(user.sid):
            raise SecurityError('fixture_security.sid_invalid')
        known_sids = []
        for kind in (22, 26):  # LocalSystem and BuiltinAdministrators, no names exported
            sid_buffer = (ctypes.c_ubyte * 68)()
            sid_size = ctypes.c_uint32(len(sid_buffer))
            if not api.CreateWellKnownSid(kind, None, sid_buffer, ctypes.byref(sid_size)):
                raise SecurityError('fixture_security.sid_query_failed')
            if not 8 <= sid_size.value <= len(sid_buffer):
                raise SecurityError('fixture_security.sid_bound')
            _sid_in_buffer(ctypes.addressof(sid_buffer), ctypes.addressof(sid_buffer), sid_size.value)
            if not api.IsValidSid(sid_buffer):
                raise SecurityError('fixture_security.sid_invalid')
            known_sids.append(sid_buffer)  # caller-owned buffers kept alive through queries
        result = {}
        for label, handle in (('root', prepared.root), ('file', prepared.file)):
            descriptor, owner, dacl = pointer(), pointer(), pointer()
            try:
                # SE_FILE_OBJECT, OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION.
                if api.GetSecurityInfo(handle, 1, 5, ctypes.byref(owner), None,
                                       ctypes.byref(dacl), None, ctypes.byref(descriptor)):
                    raise SecurityError('fixture_security.descriptor_query_failed')
                if not descriptor.value:
                    raise SecurityError('fixture_security.descriptor_query_failed')
                size = api.GetSecurityDescriptorLength(descriptor)
                if not 20 <= size <= 65536:
                    raise SecurityError('fixture_security.descriptor_bound')
                _sid_in_buffer(owner.value, descriptor.value, size)
                if not api.IsValidSid(owner):
                    raise SecurityError('fixture_security.sid_invalid')
                present, defaulted = ctypes.c_int(), ctypes.c_int()
                if not api.GetSecurityDescriptorDacl(descriptor, ctypes.byref(present),
                                                     ctypes.byref(dacl), ctypes.byref(defaulted)):
                    raise SecurityError('fixture_security.dacl_query_failed')
                row = {'owner_matches_process_user': bool(api.EqualSid(owner, user.sid)),
                       'dacl_present': bool(present.value), 'null_dacl': not bool(dacl.value),
                       'ace_count': 0}
                if present.value and dacl.value:
                    if not descriptor.value <= dacl.value or dacl.value + 8 > descriptor.value + size:
                        raise SecurityError('fixture_security.acl_bound')
                    info = AclSize()
                    if not api.GetAclInformation(dacl, ctypes.byref(info), ctypes.sizeof(info), 2):
                        raise SecurityError('fixture_security.dacl_query_failed')
                    row.update(acl_summary(info.count, info.used, info.free))
                    if dacl.value + info.used + info.free > descriptor.value + size:
                        raise SecurityError('fixture_security.acl_bound')
                    header = AclHeader.from_address(dacl.value)
                    if header.size != info.used + info.free or header.count != info.count:
                        raise SecurityError('fixture_security.acl_bound')
                    row['aces'] = _ace_observations(api, dacl.value, header.size, info.count,
                                                   user.sid, known_sids[0], known_sids[1])
                else:
                    row['aces'] = None
                result[label] = row
            finally:
                if descriptor.value and kernel.LocalFree(descriptor):
                    raise SecurityError('fixture_security.free_failed')
        return {'authority': 'none', 'observation_only': True, **result}
    finally:
        if token.value and not kernel.CloseHandle(token):
            raise SecurityError('fixture_security.token_close_failed')
