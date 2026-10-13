"""Disposable loopback MCP carrier proof, never accepts external URL/credentials."""
import asyncio
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
from smoke import client, structured
from probe_core import DUMMY_TOKENS, DUMMY_HEADERS


async def initial(url):
    async with client(url) as session:
        listing = await session.list_tools()
        assert {t.name for t in listing.tools} == {'session_bootstrap', 'session_open', 'session_probe'}
        ticket = structured(await session.call_tool('session_bootstrap', {}))
        assert ticket['ok'] and ticket['kind'] == 'bootstrap_ticket'
        # Discard first response as if lost, then retry same ticket concurrently.
        opened = structured(await session.call_tool('session_open', {'application_handle': ticket['application_handle']}))
        again = await asyncio.gather(*(session.call_tool('session_open',
            {'application_handle': ticket['application_handle']}) for _ in range(2)))
        repeated = [structured(item) for item in again]
        assert all(item['ok'] and item['reused'] for item in repeated)
        assert all(item['application_handle'] == opened['application_handle'] and
                   item['synthetic_session_id'] == opened['synthetic_session_id'] for item in repeated)
    async with client(url, 'B') as other:
        wrong = structured(await other.call_tool('session_probe', {'application_handle': opened['application_handle']}))
        assert not wrong['ok']
    async with client(url) as reconnected:
        resolved = [structured(await reconnected.call_tool('session_probe',
                    {'application_handle': opened['application_handle']})) for _ in range(2)]
        assert all(item['ok'] and item['synthetic_session_id'] == opened['synthetic_session_id'] for item in resolved)
    return ticket, opened, [ticket, opened, *repeated, wrong, *resolved]


async def after_close(url, ticket, opened):
    async with client(url) as session:
        old = structured(await session.call_tool('session_probe', {'application_handle': opened['application_handle']}))
        retry = structured(await session.call_tool('session_open', {'application_handle': ticket['application_handle']}))
        assert not old['ok'] and not retry['ok']
        fresh = structured(await session.call_tool('session_bootstrap', {}))
        new = structured(await session.call_tool('session_open', {'application_handle': fresh['application_handle']}))
        assert new['ok'] and new['synthetic_session_id'] != opened['synthetic_session_id']
        return old, retry, fresh, new


def main():
    with socket.socket() as sock:
        sock.bind(('127.0.0.1', 0))
        port = sock.getsockname()[1]
    env = {**os.environ, 'OPTIC_SESSION_PROBE_PORT': str(port), 'OPTIC_SESSION_PROBE_TUNNEL': '0'}
    process = subprocess.Popen([sys.executable, str(ROOT / 'bootstrap_server.py')], env=env,
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
        ticket, opened, proofs = asyncio.run(initial(url))
        process.stdin.write('close-a\n')
        process.stdin.flush()
        ack = acks.get(timeout=5)
        assert ack['ok'] and ack['operation'] == 'close'
        proofs.extend(asyncio.run(after_close(url, ticket, opened)))
    finally:
        process.kill()
        process.wait(timeout=10)
        reader.join(timeout=2)
    joined = ''.join(logs)
    for data in proofs:
        invocation = data['probe_invocation']
        assert invocation['marker'] in joined
        assert data['request_observation']['matches_dummy']
        if 'application_handle' in data:
            assert data['application_handle'] not in joined
    for raw in list(DUMMY_TOKENS.values()) + list(DUMMY_HEADERS.values()):
        assert raw.decode() not in joined
    print('PASS: actual three-tool MCP, same-ticket duplicate open/same synthetic session,')
    print('repeated resolution after reconnect, B denial without consumption, local close blocks replay/reopen,')
    print('explicit fresh ticket creates new empty session; marker matches and no raw tokens/handles in logs.')
    print('Local synthetic proof only, not production exactly-once effects or FranceStudent behavior.')


if __name__ == '__main__':
    main()
