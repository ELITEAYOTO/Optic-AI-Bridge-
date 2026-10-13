"""Local-only real SDK parallel and sequential controls. No external URL accepted."""
import asyncio
import json
import os
from pathlib import Path
import socket
import subprocess
import sys
import threading
import time

ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT.parent / 'francestudent-probe'))
from smoke import client, structured
from probe_core import DUMMY_TOKENS, DUMMY_HEADERS


async def exercise(url):
    proof = []
    tokens = []
    async with client(url) as session:
        tools = await session.list_tools()
        assert {tool.name for tool in tools.tools} == {'parallel_create_group', 'parallel_wait', 'parallel_report'}
        first = structured(await session.call_tool('parallel_create_group', {}))
        assert first['ok']
        handle = first['application_handle']
        tokens.append(handle)
        proof.append(first)
        waited = await asyncio.gather(*(session.call_tool('parallel_wait', {'application_handle': handle}) for _ in range(2)))
        results = [structured(item) for item in waited]
        assert all(item['ok'] and item['overlap_observed'] and item['state'] == 'completed' for item in results)
        assert sorted(item['active_on_entry'] for item in results) == [1, 2]
        proof.extend(results)
        report = structured(await session.call_tool('parallel_report', {'application_handle': handle}))
        assert report['ok'] and report['completed_pair_overlap'] and report['active'] == 0
        assert len(report['records']) == 2
        assert {item['invocation_id'] for item in report['records']} == {item['invocation_id'] for item in results}
        for item in results:
            assert item['request_sequence'] == item['probe_invocation']['request_sequence']
        proof.append(report)
    # New carrier, same owner/group. Context reconnect is not identity/authority.
    async with client(url, 'B') as other:
        denied = structured(await other.call_tool('parallel_report', {'application_handle': handle}))
        assert not denied['ok']
        proof.append(denied)
    async with client(url) as session:
        again = structured(await session.call_tool('parallel_report', {'application_handle': handle}))
        assert again['ok'] and again['records'] == report['records']
        proof.append(again)
        sequential = structured(await session.call_tool('parallel_create_group', {}))
        proof.append(sequential)
        tokens.append(sequential['application_handle'])
        for _ in range(2):
            one = structured(await session.call_tool('parallel_wait', {'application_handle': sequential['application_handle']}))
            assert one['ok'] and not one['overlap_observed']
            proof.append(one)
        control = structured(await session.call_tool('parallel_report', {'application_handle': sequential['application_handle']}))
        assert not control['completed_pair_overlap'] and control['active'] == 0
        proof.append(control)
    return proof, tokens


def main():
    with socket.socket() as sock:
        sock.bind(('127.0.0.1', 0))
        port = sock.getsockname()[1]
    env = {**os.environ, 'OPTIC_GROUPED_PORT': str(port), 'OPTIC_GROUPED_TUNNEL': '0'}
    process = subprocess.Popen([sys.executable, str(ROOT / 'server.py')], env=env,
        stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    logs = []
    def drain():
        for line in process.stdout:
            if len(logs) < 2048:
                logs.append(line[:4096])
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
        proof, tokens = asyncio.run(exercise(f'http://127.0.0.1:{port}/mcp'))
        deadline = time.monotonic() + 2
        while not all(item['probe_invocation']['marker'] in ''.join(logs) for item in proof):
            assert time.monotonic() < deadline, 'accepted proof writer deadline'
            time.sleep(.01)
    finally:
        process.kill()
        process.wait(timeout=10)
        reader.join(timeout=2)
    joined = ''.join(logs)
    events = [json.loads(line[len('[GROUP] '):]) for line in logs if line.startswith('[GROUP] ')]
    for item in proof:
        assert item['probe_invocation']['marker'] in joined
        assert item['probe_invocation']['console_queued']
        if 'invocation_id' in item:
            matching = [e for e in events if e['invocation_id'] == item['invocation_id']]
            assert {e['phase'] for e in matching} == {'enter', 'exit'}
            assert all(e['group_id'] == item['group_id'] and e['request_sequence'] == item['request_sequence'] for e in matching)
    for raw in tokens + [value.decode() for value in list(DUMMY_TOKENS.values()) + list(DUMMY_HEADERS.values())]:
        assert raw not in joined
    print('PASS: exact3-tool MCP, same-group concurrent pair measured overlap, sequential control false,')
    print('owner B denied, fresh A transport reports stable group; ENTER/EXIT and TOOL proofs matched; no raw credentials/group tokens.')
    print('Local SDK proof only. FranceStudent grouped temporal overlap remains unmeasured.')


if __name__ == '__main__':
    main()
