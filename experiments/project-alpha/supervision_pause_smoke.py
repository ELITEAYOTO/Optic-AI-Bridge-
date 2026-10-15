"""Suspend only an owned generated-test core; a missed lease cannot resurrect."""
import ctypes
import json
import os
from pathlib import Path
import queue
import signal
import subprocess
import sys
import tempfile
import threading
import time


def pause(child, suspend):
    assert child.poll() is None
    if os.name == 'nt':
        from ctypes import wintypes
        operation = ctypes.WinDLL('ntdll').NtSuspendProcess if suspend else ctypes.WinDLL('ntdll').NtResumeProcess
        operation.argtypes = [wintypes.HANDLE]
        operation.restype = ctypes.c_long
        # This handle is created and owned by this exact Popen instance.
        if operation(int(child._handle)) != 0:
            raise AssertionError('owned test process suspension failed')
    else:
        os.kill(child.pid, signal.SIGSTOP if suspend else signal.SIGCONT)


def main(binary):
    with tempfile.TemporaryDirectory(prefix='optic-paused-supervisor-') as directory:
        root = Path(directory)
        (root / 'public.txt').write_bytes(b'PUBLIC_PAUSE_FIXTURE\n')
        with tempfile.TemporaryFile() as diagnostic:
            child = subprocess.Popen([str(Path(binary).resolve(strict=True)), str(root),
                '--session-lifetime-seconds', '5', '--supervise-session'],
                stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=diagnostic)
            responses = queue.Queue(maxsize=8)
            def read():
                try:
                    for _ in range(8):
                        line = child.stdout.readline(65537)
                        if not line or len(line) > 65536:
                            responses.put_nowait(None)
                            return
                        responses.put_nowait(json.loads(line))
                except (OSError, ValueError, queue.Full):
                    try:
                        responses.put_nowait(None)
                    except queue.Full:
                        pass
            reader = threading.Thread(target=read, daemon=True)
            reader.start()
            sequence = 0
            suspended = False
            def request(method, params):
                nonlocal sequence
                sequence += 1
                payload = {'jsonrpc': '2.0', 'id': sequence, 'method': method, 'params': params}
                child.stdin.write(json.dumps(payload).encode() + b'\n')
                child.stdin.flush()
                result = responses.get(timeout=10)
                assert result and result.get('id') == sequence, 'missing/mismatched bounded response'
                return result
            try:
                initialized = request('initialize', {'protocolVersion': '2025-03-26',
                    'capabilities': {}, 'clientInfo': {'name': 'owned-pause-proof', 'version': '1'}})
                assert 'result' in initialized
                child.stdin.write(b'{"jsonrpc":"2.0","method":"notifications/initialized"}\n')
                child.stdin.flush()
                before = request('tools/call', {'name': 'session_info', 'arguments': {}})
                assert 'result' in before
                pause(child, True)
                suspended = True
                time.sleep(6.5)
                pause(child, False)
                suspended = False
                for name, arguments in (('session_info', {}), ('fs_read', {'path': 'public.txt'})):
                    denied = request('tools/call', {'name': name, 'arguments': arguments})
                    assert denied.get('error', {}).get('message') == 'optic.session_inactive', denied
                child.stdin.close()
                assert child.wait(timeout=10) == 0
            finally:
                if suspended and child.poll() is None:
                    pause(child, False)
                if child.poll() is None:
                    child.kill()
                    child.wait(timeout=10)
                child.stdin.close()
                reader.join(timeout=2)
                assert not reader.is_alive(), 'owned reader did not stop'
                child.stdout.close()
    print('SUPERVISION PAUSE PASS: missed lease stays inactive after owned process resume; stdio EOF exits cleanly.')


if __name__ == '__main__':
    main(sys.argv[1])
