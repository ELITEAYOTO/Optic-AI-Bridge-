"""Windows test helper: pin only the core child of a Popen we created."""
import ctypes
import re
from ctypes import wintypes
from pathlib import Path


class Entry(ctypes.Structure):
    _fields_ = [('size', wintypes.DWORD), ('usage', wintypes.DWORD), ('pid', wintypes.DWORD),
                ('heap', ctypes.c_void_p), ('module', wintypes.DWORD), ('threads', wintypes.DWORD),
                ('parent', wintypes.DWORD), ('priority', wintypes.LONG), ('flags', wintypes.DWORD),
                ('name', wintypes.WCHAR * 260)]


class Basic(ctypes.Structure):
    _fields_ = [('status', wintypes.LONG), ('peb', ctypes.c_void_p), ('affinity', ctypes.c_size_t),
                ('priority', wintypes.LONG), ('pid', ctypes.c_size_t), ('parent', ctypes.c_size_t)]


class OwnedCore:
    def __init__(self, parent, binary):
        assert parent.poll() is None
        kernel = ctypes.WinDLL('kernel32', use_last_error=True)
        kernel.CreateToolhelp32Snapshot.argtypes = [wintypes.DWORD, wintypes.DWORD]
        kernel.CreateToolhelp32Snapshot.restype = wintypes.HANDLE
        kernel.Process32FirstW.argtypes = kernel.Process32NextW.argtypes = [wintypes.HANDLE, ctypes.POINTER(Entry)]
        kernel.Process32FirstW.restype = kernel.Process32NextW.restype = wintypes.BOOL
        kernel.CloseHandle.argtypes = [wintypes.HANDLE]
        kernel.OpenProcess.argtypes = [wintypes.DWORD, wintypes.BOOL, wintypes.DWORD]
        kernel.OpenProcess.restype = wintypes.HANDLE
        kernel.QueryFullProcessImageNameW.argtypes = [wintypes.HANDLE, wintypes.DWORD, wintypes.LPWSTR, ctypes.POINTER(wintypes.DWORD)]
        kernel.QueryFullProcessImageNameW.restype = wintypes.BOOL
        kernel.WaitForSingleObject.argtypes = [wintypes.HANDLE, wintypes.DWORD]
        kernel.WaitForSingleObject.restype = wintypes.DWORD
        kernel.TerminateProcess.argtypes = [wintypes.HANDLE, wintypes.UINT]
        kernel.TerminateProcess.restype = wintypes.BOOL
        snapshot = kernel.CreateToolhelp32Snapshot(2, 0)
        assert snapshot and snapshot != ctypes.c_void_p(-1).value
        processes = {}
        try:
            entry = Entry()
            entry.size = ctypes.sizeof(entry)
            present = kernel.Process32FirstW(snapshot, ctypes.byref(entry))
            for _ in range(4096):
                if not present:
                    break
                processes[entry.pid] = (entry.parent, entry.name.lower())
                present = kernel.Process32NextW(snapshot, ctypes.byref(entry))
            else:
                raise AssertionError('bounded process snapshot exhausted')
        finally:
            kernel.CloseHandle(snapshot)
        found = []
        for pid, (ancestor, name) in processes.items():
            if name != 'optic-bridge.exe':
                continue
            chain = [(pid, ancestor)]
            for _ in range(4):
                if ancestor == parent.pid:
                    found.append(chain)
                    break
                info = processes.get(ancestor)
                if info is None or not re.fullmatch(r'python(?:[0-9]+(?:\.[0-9]+)*)?w?\.exe', info[1]):
                    break
                chain.append((ancestor, info[0]))
                ancestor = info[0]
        assert len(found) == 1, 'expected one core beneath the verified Python launcher chain'
        chain = found[0]
        query = ctypes.WinDLL('ntdll').NtQueryInformationProcess
        query.argtypes = [wintypes.HANDLE, wintypes.ULONG, ctypes.c_void_p, wintypes.ULONG, ctypes.POINTER(wintypes.ULONG)]
        query.restype = ctypes.c_long
        handles = {}
        try:
            for pid, ancestor in chain:
                handle = kernel.OpenProcess(0x100401, False, pid)
                assert handle, 'owned process handle unavailable'
                handles[pid] = handle
                basic, received = Basic(), wintypes.ULONG()
                assert query(handle, 0, ctypes.byref(basic), ctypes.sizeof(basic), ctypes.byref(received)) == 0
                assert basic.parent == ancestor and basic.pid == pid and parent.poll() is None
                buffer, size = ctypes.create_unicode_buffer(32768), wintypes.DWORD(32768)
                assert kernel.QueryFullProcessImageNameW(handle, 0, buffer, ctypes.byref(size))
                if pid == chain[0][0]:
                    assert Path(buffer.value).samefile(binary), 'owned core image mismatch'
                else:
                    assert re.fullmatch(r'python(?:[0-9]+(?:\.[0-9]+)*)?w?\.exe', Path(buffer.value).name.lower())
        except BaseException:
            for handle in handles.values():
                kernel.CloseHandle(handle)
            raise
        self.kernel, self.handle = kernel, handles[chain[0][0]]
        self.handles, self.parent = handles, parent
        self.runner_handle = handles.get(chain[0][1])

    def crash_runner(self):
        # Python's Windows venv launcher may own a second actual interpreter.
        # The pinned direct parent of the core is that test-owned runner.
        if self.runner_handle is None:
            self.parent.kill()
        else:
            assert self.kernel.TerminateProcess(self.runner_handle, 1)
        self.parent.wait(timeout=10)

    def require_exit(self):
        assert self.kernel.WaitForSingleObject(self.handle, 10000) == 0, 'core did not exit after parent death'

    def close(self):
        # Every handle was pinned and checked against our live Popen ancestry.
        for handle in reversed(tuple(self.handles.values())):
            if self.kernel.WaitForSingleObject(handle, 0) == 258:
                assert self.kernel.TerminateProcess(handle, 1)
                assert self.kernel.WaitForSingleObject(handle, 5000) == 0
            self.kernel.CloseHandle(handle)
        self.handles.clear()
        self.handle = None
