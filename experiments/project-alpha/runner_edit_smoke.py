"""Own generated runner child, real HTTP edit, stop and retained project proof."""
import asyncio
from pathlib import Path
import socket
import subprocess
import sys
import tempfile
import threading

HERE = Path(__file__).resolve().parent
for directory in ('real-link-admission', 'credential-lifecycle'):
    sys.path.insert(0, str(HERE.parent / directory))
import httpx2
from http_smoke import client
from project_results import structured
from lifecycle_core import TOKENS


async def exercise(url):
    async with asyncio.timeout(20):
        async with httpx2.AsyncClient(timeout=2) as http:
            while True:
                try:
                    if (await http.post(url, json={'method': 'ping'})).status_code == 401:
                        break
                except (httpx2.ConnectError, httpx2.ConnectTimeout):
                    pass
                await asyncio.sleep(.1)
    async with client(url, TOKENS['A'][0]) as session:
        payload = structured(await session.call_tool('optic_fs_write', {
            'path': 'src/index.html', 'content': '<h1>Saved public project</h1>', 'expected_kind': 'absent'}))
        assert payload['ok']
        observed = structured(await session.call_tool('optic_fs_read', {'path': 'src/index.html'}))
        assert observed['upstream']['content_version'] == payload['upstream']['content_version']


def run(binary):
    if sys.platform != 'win32':
        print('RUNNER EDIT SKIP: Windows mutation runner only; platform refusal tested separately.')
        return
    with tempfile.TemporaryDirectory(prefix='optic-alpha-runner-proof-') as directory:
        with socket.socket() as listener:
            listener.bind(('127.0.0.1', 0))
            port = listener.getsockname()[1]
        child = subprocess.Popen([sys.executable, str(HERE / 'project_runner.py'), '--binary',
            str(Path(binary).resolve(strict=True)), '--output-root', directory,
            '--port', str(port), '--lifetime', '30'], stdin=subprocess.PIPE,
            stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        captured = []
        lock = threading.Lock()
        def collect(stream):
            for line in iter(lambda: stream.readline(4097), ''):
                with lock:
                    if len(captured) < 256:
                        captured.append(line[:4096])
        readers = [threading.Thread(target=collect, args=(stream,), daemon=True)
                   for stream in (child.stdout, child.stderr)]
        for reader in readers:
            reader.start()
        try:
            asyncio.run(exercise(f'http://127.0.0.1:{port}/mcp'))
            child.stdin.write('stop\n')
            child.stdin.flush()
            assert child.wait(timeout=20) == 0
            for reader in readers:
                reader.join(timeout=2)
            output = ''.join(captured)
            assert '"session_revoked": true' in output and '"drained": true' in output
            assert all(token.decode() not in output for values in TOKENS.values() for token in values)
            files = list(Path(directory).glob('alpha-*/project/src/index.html'))
            assert len(files) == 1 and files[0].read_text() == '<h1>Saved public project</h1>'
        finally:
            if child.poll() is None:
                child.kill()
                child.wait(timeout=10)
            for stream in (child.stdin, child.stdout, child.stderr):
                stream.close()
    print('ALPHA RUNNER PASS: actual child startup/write/versioned read/local stop; generated project survives stop, public auth values absent from logs.')


if __name__ == '__main__':
    run(sys.argv[1])
