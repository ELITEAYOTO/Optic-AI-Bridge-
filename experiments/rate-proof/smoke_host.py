"""Disposable local HTTP/MCP proof; never accepts an external URL."""
import asyncio
import http.client
import json
import os
from pathlib import Path
import queue
import socket
import subprocess
import sys
import threading
import time

ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT.parent / 'francestudent-probe'))
from smoke import client, structured, header_map
from probe_core import DUMMY_TOKENS, DUMMY_HEADERS


async def exercise(url):
    async with client(url) as session:
        tools = await session.list_tools()
        assert {t.name for t in tools.tools} == {'rate_probe'}
        data = structured(await session.call_tool('rate_probe', {}))
        assert data['ok'] and data['authority'] == 'none'
        assert data['request_observation']['matches_dummy']
        return data['probe_invocation']


def main():
    with socket.socket() as sock:
        sock.bind(('127.0.0.1', 0))
        port = sock.getsockname()[1]
    env = {**os.environ, 'OPTIC_RATE_PORT': str(port), 'OPTIC_RATE_TUNNEL': '0'}
    process = subprocess.Popen([sys.executable, str(ROOT / 'host_server.py')], env=env,
        stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    logs, acks = [], queue.Queue(maxsize=16)
    def drain():
        for line in process.stdout:
            if len(logs) < 2048:
                logs.append(line[:4096])
            if line.startswith('[LOCAL] {'):
                acks.put_nowait(json.loads(line[len('[LOCAL] '):]))
    reader = threading.Thread(target=drain, daemon=True)
    reader.start()
    def command(value):
        process.stdin.write(value + '\n')
        process.stdin.flush()
        ack = acks.get(timeout=5)
        assert ack['operation'] == value
        return ack
    connection = None
    try:
        deadline = time.monotonic() + 20
        while True:
            assert process.poll() is None
            try:
                with socket.create_connection(('127.0.0.1', port), timeout=.2):
                    break
            except OSError:
                assert time.monotonic() < deadline
                time.sleep(.05)
        url = f'http://127.0.0.1:{port}/mcp'
        first = asyncio.run(exercise(url))
        ack = command('hold-rate')
        connection = http.client.HTTPConnection('127.0.0.1', port, timeout=5)
        for index in range(40):
            headers = header_map() if index % 2 else {'Authorization': 'Bearer NEVER-LOG-RATE-CANARY'}
            connection.request('POST', '/mcp', body=b'{}', headers=headers)
            response = connection.getresponse()
            assert response.status == 429
            assert response.getheader('Retry-After') == '1'
            assert json.loads(response.read()) == {'code': 'probe.rate_limited'}
        command('release-rate')
        second = asyncio.run(exercise(url))
        assert first['server_instance'] == second['server_instance'] == ack['server_instance']
        connection.request('POST', '/mcp', body=json.dumps({'method': 'tools/call',
            'params': {'name': 'unknown_synthetic_probe', 'arguments': {}}}), headers=header_map())
        response = connection.getresponse()
        assert response.status == 400
        response.read()
        connection.request('POST', '/mcp', body=b'{}')
        response = connection.getresponse()
        assert response.status == 401
        response.read()
    finally:
        if connection:
            connection.close()
        process.kill()
        process.wait(timeout=10)
        reader.join(timeout=2)
    joined = ''.join(logs)
    for proof in (first, second):
        assert proof['marker'] in joined
    assert joined.count('[RATE] ') == 32
    assert 'NEVER-LOG-RATE-CANARY' not in joined
    for raw in list(DUMMY_TOKENS.values()) + list(DUMMY_HEADERS.values()):
        assert raw.decode() not in joined
    print('PASS: exact one-tool MCP, local hold -> HTTP429 for valid/invalid dummy requests,')
    print('bounded32 refusal logs, local release -> fresh MCP success on same instance; no credential values.')
    print('Local forced-rejection proof only; natural refill is deterministic unit proof, host behavior unmeasured.')


if __name__ == '__main__':
    main()
