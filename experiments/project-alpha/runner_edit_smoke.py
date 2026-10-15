"""Own generated runner child, real HTTP edit, stop and retained project proof."""
import asyncio
import hashlib
import json
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


async def exercise(url, node_mode=False):
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
        if node_mode:
            style = structured(await session.call_tool('optic_fs_write', {
                'path': 'src/style.css', 'content': 'body{font-family:system-ui}', 'expected_kind': 'absent'}))
            assert style['ok'], style
            from node_http_smoke import finished
            names = {tool.name for tool in (await session.list_tools()).tools}
            from project_schema import TOOLS
            from execution_gate import TASK_TOOLS
            assert names == set(TOOLS + TASK_TOOLS)
            snapshot = structured(await session.call_tool('optic_fs_read', {'path': 'src/model.js'}))
            broken = structured(await session.call_tool('optic_fs_write', {
                'path': 'src/model.js', 'content': 'module.exports.add=(a,b)=>a-b;',
                'expected_kind': 'content', 'expected_version': snapshot['upstream']['content_version']}))
            assert broken['ok'], broken
            started = structured(await session.call_tool('optic_task_start', {'profile': 'test'}))
            assert started['ok'], started
            job = started['upstream']['job_id']
            failure = await finished(session, job)
            assert failure['status'] == 'exited' and failure['exit_code'] != 0, failure
            error = structured(await session.call_tool('optic_task_read', {'job_id': job, 'stream': 'stderr'}))
            assert 'AssertionError' in error['upstream']['text'], error
            fixed = structured(await session.call_tool('optic_fs_write', {
                'path': 'src/model.js', 'content': 'module.exports.add=(a,b)=>a+b;',
                'expected_kind': 'content', 'expected_version': broken['upstream']['content_version']}))
            assert fixed['ok'], fixed
            for profile in ('check', 'test'):
                started = structured(await session.call_tool('optic_task_start', {'profile': profile}))
                assert started['ok'], started
                job = started['upstream']['job_id']
                result = await finished(session, job)
                assert result['status'] == 'exited' and result['exit_code'] == 0, result
                output = structured(await session.call_tool('optic_task_read', {'job_id': job}))
                assert 'ALPHA_' + profile.upper() + '_PASS' in output['upstream']['text'], output


async def exercise_resume(url):
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
        value = structured(await session.call_tool('optic_fs_read', {'path': 'src/model.js'}))
        assert value['upstream']['text'] == 'module.exports.add=(a,b)=>a+b;'
        from node_http_smoke import finished
        start = structured(await session.call_tool('optic_task_start', {'profile': 'test'}))
        result = await finished(session, start['upstream']['job_id'])
        assert result['status'] == 'exited' and result['exit_code'] == 0, result


def run(binary, node=None):
    if sys.platform != 'win32':
        print('RUNNER EDIT SKIP: Windows mutation runner only; platform refusal tested separately.')
        return
    with tempfile.TemporaryDirectory(prefix='optic-alpha-runner-proof-') as directory:
        with socket.socket() as listener:
            listener.bind(('127.0.0.1', 0))
            port = listener.getsockname()[1]
        arguments = [sys.executable, str(HERE / 'project_runner.py'), '--binary',
            str(Path(binary).resolve(strict=True)), '--output-root', directory,
            '--port', str(port), '--lifetime', '90' if node else '30']
        if node:
            arguments.extend(['--node', str(Path(node).resolve(strict=True))])
        child = subprocess.Popen(arguments, stdin=subprocess.PIPE,
            stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        captured = []
        lock = threading.Lock()
        def collect(stream):
            for line in iter(lambda: stream.readline(4097), ''):
                with lock:
                    if len(captured) < 1024:
                        captured.append(line[:4096])
        readers = [threading.Thread(target=collect, args=(stream,), daemon=True)
                   for stream in (child.stdout, child.stderr)]
        for reader in readers:
            reader.start()
        try:
            asyncio.run(exercise(f'http://127.0.0.1:{port}/mcp', bool(node)))
            if node:
                assert not list(Path(directory).glob('alpha-*/viewer-ready.json'))
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
            report = json.loads((files[0].parents[2] / 'last-stop.json').read_bytes())
            assert report['revocation_confirmed'] and report['work_drained']
            assert report['reason'] == 'operator'
            if node:
                assert (files[0].parent / 'model.js').read_text() == 'module.exports.add=(a,b)=>a+b;'
                ready = json.loads((files[0].parents[2] / 'viewer-ready.json').read_text())
                assert ready['version'] == 1
                assert set(ready['files']) == {'index.html', 'style.css', 'app.js', 'model.js', 'tests.js'}
                for name, version in ready['files'].items():
                    assert hashlib.sha256((files[0].parent / name).read_bytes()).hexdigest() == version
                assert '"viewer_ready": true' in output
                saved = {name: (files[0].parent / name).read_bytes() for name in ready['files']}
                resumed = subprocess.Popen(arguments + ['--resume-run', files[0].parents[2].name],
                    stdin=subprocess.PIPE, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, text=True)
                try:
                    asyncio.run(exercise_resume(f'http://127.0.0.1:{port}/mcp'))
                    assert not (files[0].parents[2] / 'viewer-ready.json').exists()
                    resumed.stdin.write('stop\n')
                    resumed.stdin.flush()
                    assert resumed.wait(timeout=20) == 0
                    assert len(list(Path(directory).glob('alpha-*'))) == 1
                    assert saved == {name: (files[0].parent / name).read_bytes() for name in saved}
                    assert (files[0].parents[2] / 'viewer-ready.json').is_file()
                    report = json.loads((files[0].parents[2] / 'last-stop.json').read_bytes())
                    assert report['revocation_confirmed'] and report['reason'] == 'operator'
                finally:
                    if resumed.poll() is None:
                        resumed.kill()
                        resumed.wait(timeout=10)
                    resumed.stdin.close()
        finally:
            if child.poll() is None:
                child.kill()
                child.wait(timeout=10)
            for stream in (child.stdin, child.stdout, child.stderr):
                stream.close()
    print('ALPHA RUNNER PASS: actual child startup/write/versioned read/local stop; generated project survives stop, public auth values absent from logs.')
    if node:
        print('NODE RUNNER PASS: exact ten-tool startup, native relative module failing assertion -> versioned fix -> check/test success; no build/server/dependency claim.')


if __name__ == '__main__':
    run(sys.argv[1], sys.argv[2] if len(sys.argv) == 3 else None)
