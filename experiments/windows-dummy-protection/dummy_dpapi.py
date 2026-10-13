"""Native CurrentUser API proof for ONE fixed public dummy-verifier fixture.

No real token input, file selector, network listener or general decrypt interface.
"""
import ctypes
import hashlib
import hmac
import json
import sys

MAGIC = b'OPTIC-PUBLIC-DUMMY-DPAPI-V1\0'
ENTROPY = b'Optic-public-dummy-verifier-fixture-only-v1'
MAX_BLOB = 16 * 1024
UI_FORBIDDEN = 1
PUBLIC_DUMMIES = (b'optic-probe-dummy-owner-a', b'optic-probe-dummy-owner-b')
PAYLOAD = json.dumps({'schema': 'public-dummy-verifier-v1', 'verifiers': {
    'A': hashlib.sha256(PUBLIC_DUMMIES[0]).hexdigest(),
    'B': hashlib.sha256(PUBLIC_DUMMIES[1]).hexdigest()}}, sort_keys=True, separators=(',', ':')).encode('ascii')


class FixtureError(Exception):
    def __init__(self, code):
        self.code = code
        super().__init__(code)


class Blob(ctypes.Structure):
    # DWORD is fixed32bit even on a64bit Windows process.
    _fields_ = [('size', ctypes.c_uint32), ('data', ctypes.POINTER(ctypes.c_ubyte))]


def _blob(data):
    keepalive = (ctypes.c_ubyte * len(data)).from_buffer_copy(data)
    return Blob(len(data), ctypes.cast(keepalive, ctypes.POINTER(ctypes.c_ubyte))), keepalive


def _native(data, protect):
    if sys.platform != 'win32':
        raise FixtureError('dummy_dpapi.windows_required')
    # Fixed libraries/entrypoints, UI forbidden, no LOCAL_MACHINE flag or prompt.
    try:
        # LOAD_LIBRARY_SEARCH_SYSTEM32: no current-directory DLL selection.
        crypt = ctypes.WinDLL('crypt32.dll', use_last_error=True, winmode=0x800)
        kernel = ctypes.WinDLL('kernel32.dll', use_last_error=True, winmode=0x800)
    except OSError:
        raise FixtureError('dummy_dpapi.native_unavailable') from None
    function = crypt.CryptProtectData if protect else crypt.CryptUnprotectData
    function.argtypes = [ctypes.POINTER(Blob), ctypes.c_wchar_p if protect else ctypes.c_void_p,
        ctypes.POINTER(Blob), ctypes.c_void_p, ctypes.c_void_p, ctypes.c_uint32, ctypes.POINTER(Blob)]
    function.restype = ctypes.c_int
    kernel.LocalFree.argtypes = [ctypes.c_void_p]
    kernel.LocalFree.restype = ctypes.c_void_p
    source, source_buffer = _blob(data)
    entropy, entropy_buffer = _blob(ENTROPY)
    output = Blob()
    try:
        succeeded = function(ctypes.byref(source), None, ctypes.byref(entropy), None, None,
                             UI_FORBIDDEN, ctypes.byref(output))
        if not succeeded:
            raise FixtureError('dummy_dpapi.protect_failed' if protect else 'dummy_dpapi.unprotect_failed')
        if not output.data or not 0 < output.size <= MAX_BLOB:
            raise FixtureError('dummy_dpapi.output_bound')
        return ctypes.string_at(output.data, output.size)
    finally:
        if output.data:
            # Native allocated output freed on success/failure. Public fixture only;
            # this does not promise Python immutable byte copies are zeroized.
            if output.size <= MAX_BLOB:
                ctypes.memset(output.data, 0, output.size)
            if kernel.LocalFree(ctypes.cast(output.data, ctypes.c_void_p)):
                raise FixtureError('dummy_dpapi.native_free_failed')


def protect_public_fixture():
    encrypted = _native(PAYLOAD, True)
    if len(MAGIC) + len(encrypted) > MAX_BLOB:
        raise FixtureError('dummy_dpapi.output_bound')
    return MAGIC + encrypted


def check_public_fixture(blob):
    if not isinstance(blob, bytes) or not len(MAGIC) < len(blob) <= MAX_BLOB or not blob.startswith(MAGIC):
        raise FixtureError('dummy_dpapi.invalid_fixture')
    plain = _native(blob[len(MAGIC):], False)
    if not hmac.compare_digest(plain, PAYLOAD):
        raise FixtureError('dummy_dpapi.unexpected_payload')
    # Plaintext never returned to caller or printed. Fixed fixture-match bit only.
    return {'ok': True, 'fixture': 'public_dummy_verifier_v1', 'authority': 'none'}
