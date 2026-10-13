"""Loopback HTTP -> real Rust stdio with generated files and public dummy auth."""
import asyncio
import base64
from contextlib import asynccontextmanager
import json
import logging
from pathlib import Path
import socket
import sys
import tempfile

import httpx2
import uvicorn
from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client
from mcp.client.streamable_http import streamable_http_client
from mcp.shared.exceptions import MCPError
from real_smoke import structured
from lifecycle_core import Registry, TOKENS
from forward_core import ForwardGate
from http_adapter import make_app, local_stop
from probe_core import DUMMY_HEADERS


def headers(token):
    return {'Authorization': 'Bearer ' + token.decode(),
            **{key.decode(): value.decode() for key, value in DUMMY_HEADERS.items()}}


@asynccontextmanager
async def client(url, token):
    async with httpx2.AsyncClient(headers=headers(token), timeout=10) as http:
        async with streamable_http_client(url, http_client=http) as (read, write):
            async with ClientSession(read, write, read_timeout_seconds=10) as session:
                await session.initialize()
                yield session


async def denied(url, token, status=401, body=None):
    async with httpx2.AsyncClient(headers=headers(token), timeout=5) as http:
        response = await http.post(url, json=body or {'jsonrpc': '2.0', 'id': 1,
            'method': 'initialize', 'params': {}})
        assert response.status_code == status


async def run(binary):
    logs = []
    binary = Path(binary).resolve(strict=True)
    with tempfile.TemporaryDirectory(prefix='optic-http-fixed-proof-') as directory:
        root = Path(directory)
        (root / 'public').mkdir()
        fixture = b'PUBLIC_GENERATED_HTTP_FIXTURE\n'
        (root / 'public' / 'hello.txt').write_bytes(fixture)
        async with stdio_client(StdioServerParameters(command=str(binary), args=[str(root)])) as (read, write):
            async with ClientSession(read, write) as upstream:
                await upstream.initialize()
                assert 'session_cancel' in {item.name for item in (await upstream.list_tools()).tools}
                state = Registry()
                identity = state.authenticate(b'Bearer ' + TOKENS['A'][0])
                async def forward(name, arguments):
                    return structured(await upstream.call_tool(name, arguments))
                gate = ForwardGate(state, identity, forward)
                app = make_app(gate, state, emit=logs.append)
                listener = socket.socket()
                listener.bind(('127.0.0.1', 0))
                listener.listen(32)
                url = f'http://127.0.0.1:{listener.getsockname()[1]}/mcp'
                service = uvicorn.Server(uvicorn.Config(app, log_config=None,
                    access_log=False, limit_concurrency=32, timeout_graceful_shutdown=5))
                task = asyncio.create_task(service.serve(sockets=[listener]))
                try:
                    async with asyncio.timeout(10):
                        while not service.started:
                            if task.done():
                                await task
                                raise AssertionError('HTTP startup failed')
                            await asyncio.sleep(.01)
                    async with client(url, TOKENS['A'][0]) as session:
                        assert {item.name for item in (await session.list_tools()).tools} == {
                            'optic_session_info', 'optic_fixture_list', 'optic_fixture_read'}
                        initial = structured(await session.call_tool('optic_session_info', {}))
                        listed = structured(await session.call_tool('optic_fixture_list', {}))
                        assert listed['ok'] and len(listed['upstream']['entries']) == 1
                        payload = structured(await session.call_tool('optic_fixture_read', {}))
                        assert payload['ok'] and base64.b64decode(payload['upstream']['data']) == fixture
                    await denied(url, TOKENS['B'][0])
                    await denied(url, TOKENS['A'][0], 400, {'jsonrpc': '2.0', 'id': 2,
                        'method': 'tools/call', 'params': {'name': 'optic_fixture_read',
                        'arguments': {'path': 'CANARY_CALLER_PATH'}}})
                    state.control('rotate', 'A')
                    await denied(url, TOKENS['A'][0])
                    async with client(url, TOKENS['A'][1]) as session:
                        repeated = structured(await session.call_tool('optic_session_info', {}))
                        assert repeated['upstream'] == initial['upstream']
                    state.control('revoke', 'A')
                    await denied(url, TOKENS['A'][1])
                    assert await local_stop(gate, upstream) == {
                        'ok': True, 'drained': True, 'session_revoked': True}
                    try:
                        await upstream.call_tool('fs_read', {'path': 'public/hello.txt', 'max_bytes': 1024})
                    except MCPError as error:
                        assert error.message == 'optic.session_inactive'
                    else:
                        raise AssertionError('real revoked session read was not denied')
                    await denied(url, TOKENS['A'][1])
                    capture = '\n'.join(logs)
                    for token in (item for values in TOKENS.values() for item in values):
                        assert token.decode() not in capture
                    assert 'CANARY_CALLER_PATH' not in capture and str(root) not in capture
                    tools = [json.loads(line.split(' TOOL ', 1)[1]) for line in logs if ' TOOL ' in line]
                    assert len(tools) == 4
                finally:
                    service.should_exit = True
                    await asyncio.wait_for(task, 10)
                    listener.close()
                    await gate.stop()
    print('PASS: loopback HTTP -> real Rust fixed fixture, exact allowlist, owner/args/old/revoked denial, reconnect stable session, local actual session revoke, redacted proofs.')
    print('Generated local fixture only; no FranceStudent live test or user-project alpha claim.')


if __name__ == '__main__':
    logging.disable(logging.CRITICAL)
    asyncio.run(run(sys.argv[1]))
