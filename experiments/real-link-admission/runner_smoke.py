"""Actual manual runner child startup/control/shutdown on generated files only."""
import asyncio
import json
from pathlib import Path
import socket
import subprocess
import sys
import threading
import time

import httpx2
from http_smoke import client, structured, headers
from lifecycle_core import TOKENS


async def exercise(url):
    async with asyncio.timeout(20):
        async with httpx2.AsyncClient(headers=headers(TOKENS['A'][0]), timeout=2) as http:
            while True:
                try:
                    response = await http.get(url)
                    if response.status_code == 405:
                        break
                except httpx2.ConnectError:
                    pass
                await asyncio.sleep(.1)
    async with client(url, TOKENS['A'][0]) as session:
        payload = structured(await session.call_tool('optic_fixture_read', {}))
        assert payload['ok'] and payload['fixture_only']


def run(binary):
    with socket.socket() as listener:
        listener.bind(('127.0.0.1', 0))
        port = listener.getsockname()[1]
    process = subprocess.Popen([sys.executable, str(Path(__file__).with_name('host_runner.py')),
        '--binary', str(Path(binary).resolve(strict=True)), '--port', str(port), '--lifetime', '30'],
        stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    captured = []
    lock = threading.Lock()
    def collect(stream):
        for line in iter(lambda: stream.readline(4097), ''):
            with lock:
                if len(captured) < 256:
                    captured.append(line[:4096])
    readers = [threading.Thread(target=collect, args=(stream,), daemon=True)
        for stream in (process.stdout, process.stderr)]
    for reader in readers:
        reader.start()
    try:
        asyncio.run(exercise(f'http://127.0.0.1:{port}/mcp'))
        process.stdin.write('stop\n')
        process.stdin.flush()
        assert process.wait(timeout=20) == 0
        for reader in readers:
            reader.join(timeout=2)
        output = ''.join(captured)
        records = [json.loads(line.split('[LOCAL] ', 1)[1]) for line in output.splitlines()
            if line.startswith('[LOCAL] ')]
        assert any(record.get('operation') == 'stop' and record.get('session_revoked')
            and record.get('drained') and record.get('ok') for record in records)
        assert all(value.decode() not in output for values in TOKENS.values() for value in values)
    finally:
        if process.poll() is None:
            process.kill()
            process.wait(timeout=10)
        for stream in (process.stdin, process.stdout, process.stderr):
            stream.close()
    print('RUNNER PASS: generated fixture startup, actual MCP read, local stop acknowledgement and clean child exit; no dummy credential values in logs.')


if __name__ == '__main__':
    run(sys.argv[1])
