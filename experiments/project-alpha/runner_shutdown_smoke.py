"""Owned Windows runner exits under expiry and stalled/broken diagnostic pipes."""
import json
import os
from pathlib import Path
import socket
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.request

HERE = Path(__file__).resolve().parent


def exercise(binary, mode):
    with tempfile.TemporaryDirectory(prefix='optic-shutdown-proof-') as directory:
        with socket.socket() as listener:
            listener.bind(('127.0.0.1', 0))
            port = listener.getsockname()[1]
        lifetime = '8' if mode.endswith('expiry') else '40'
        with tempfile.TemporaryFile() as errors:
            child = subprocess.Popen([sys.executable, str(HERE / 'project_runner.py'),
                '--binary', str(binary), '--output-root', directory, '--port', str(port),
                '--lifetime', lifetime], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=errors)
            http = urllib.request.build_opener(urllib.request.ProxyHandler({}))
            try:
                deadline = time.monotonic() + 15
                while True:
                    try:
                        http.open(urllib.request.Request(f'http://127.0.0.1:{port}/mcp', data=b'{}'), timeout=.5)
                    except urllib.error.HTTPError as error:
                        if error.code == 401:
                            break
                    except (OSError, TimeoutError):
                        pass
                    if child.poll() is not None or time.monotonic() >= deadline:
                        raise AssertionError('runner startup unavailable')
                    time.sleep(.05)
                roots = list(Path(directory).glob('alpha-*'))
                assert len(roots) == 1
                source = roots[0] / 'project/src/README.txt'
                retained = source.read_bytes()
                if mode.startswith('blocked'):
                    # Do not consume stdout: force the native proof sink to block.
                    for _ in range(100):
                        try:
                            http.open(urllib.request.Request(f'http://127.0.0.1:{port}/mcp', data=b'{}'), timeout=1)
                        except urllib.error.HTTPError as error:
                            assert error.code == 401
                if mode == 'broken-stop':
                    child.stdout.close()
                if mode.endswith('stop'):
                    child.stdin.write(b'stop\n')
                    child.stdin.flush()
                rc = child.wait(timeout=20)
                errors.seek(0)
                diagnostic = errors.read(8192)
                assert rc == 0, (mode, rc, diagnostic)
                assert b'_enter_buffered_busy' not in diagnostic
                report = json.loads((roots[0] / 'last-stop.json').read_bytes())
                assert report['reason'] == ('expiry' if mode.endswith('expiry') else 'operator'), report
                assert report['work_drained'] and report['revocation_confirmed'], report
                if mode.startswith('blocked'):
                    assert not report['diagnostics_drained'], report
                if mode == 'broken-stop':
                    assert report['diagnostics_failed'], report
                assert source.read_bytes() == retained
                assert not (roots[0] / 'viewer-ready.json').exists()
            finally:
                if child.poll() is None:
                    child.kill()
                    child.wait(timeout=5)
                child.stdin.close()
                child.stdout.close()


if __name__ == '__main__':
    if os.name != 'nt':
        print('SHUTDOWN SKIP: native Windows runner proof.')
    else:
        for case in ('normal-stop', 'normal-expiry', 'blocked-stop', 'blocked-expiry', 'broken-stop'):
            exercise(Path(sys.argv[1]).resolve(strict=True), case)
            print('SHUTDOWN PASS:', case)
