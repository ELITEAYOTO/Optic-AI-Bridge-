"""Actual Rust stdio proof with generated files; no HTTP or real workspace input."""
import asyncio
import base64
import json
from pathlib import Path
import sys
import tempfile

from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / 'credential-lifecycle'))
from lifecycle_core import Registry, TOKENS
from forward_core import ForwardGate, ForwardError


def structured(result, max_bytes=16384):
    if type(max_bytes) is not int or not 1 <= max_bytes <= 128 * 1024:
        raise ForwardError('link.invalid_output_bound')
    if result.is_error:
        raise ForwardError('link.upstream_tool_error')
    for attr in ('structured_content', 'structuredContent'):
        value = getattr(result, attr, None)
        if isinstance(value, dict):
            encoded = json.dumps(value, allow_nan=False).encode()
            if len(encoded) > max_bytes:
                raise ForwardError('link.output_bound')
            return value
    for item in result.content:
        text = getattr(item, 'text', None)
        if isinstance(text, str) and len(text.encode()) <= max_bytes:
            value = json.loads(text)
            if isinstance(value, dict):
                return value
    raise ForwardError('link.output_shape')


async def run(binary):
    # CLI chooses only the fixed test binary; workspace always generated here.
    binary = Path(binary).resolve(strict=True)
    if not binary.is_file():
        raise ValueError('link.test_binary_missing')
    with tempfile.TemporaryDirectory(prefix='optic-fixed-read-proof-') as directory:
        root = Path(directory)
        (root / 'public').mkdir()
        fixture = b'OPTIC_FIXED_READ_GENERATED_ONLY\n'
        (root / 'public' / 'hello.txt').write_bytes(fixture)
        params = StdioServerParameters(command=str(binary), args=[str(root)])
        async with stdio_client(params) as (read, write):
            async with ClientSession(read, write) as upstream:
                await upstream.initialize()
                names = {tool.name for tool in (await upstream.list_tools()).tools}
                assert {'session_info', 'fs_list', 'fs_read'} <= names
                state = Registry()
                a = state.authenticate(b'Bearer ' + TOKENS['A'][0])
                b = state.authenticate(b'Bearer ' + TOKENS['B'][0])
                async def forward(name, arguments):
                    return structured(await upstream.call_tool(name, arguments))
                gate = ForwardGate(state, a, forward)
                session = await gate.call(a, 'optic_session_info')
                capabilities = set(session.get('capabilities', []))
                assert {'file_read', 'file_search'} <= capabilities
                assert not capabilities.intersection({'file_write', 'file_delete', 'process_run', 'git_integrate', 'git_read'})
                listing = await gate.call(a, 'optic_fixture_list')
                assert [entry['path'] for entry in listing['entries']] == ['public/hello.txt']
                pair = await asyncio.gather(*(gate.call(a, 'optic_fixture_read') for _ in range(2)))
                assert all(base64.b64decode(item['data'], validate=True) == fixture for item in pair)
                try:
                    await gate.call(b, 'optic_fixture_read')
                except ForwardError:
                    pass
                else:
                    raise AssertionError('other owner admitted')
                state.control('rotate', 'A')
                try:
                    await gate.call(a, 'optic_session_info')
                except ForwardError:
                    pass
                else:
                    raise AssertionError('stale epoch admitted')
                fresh = state.authenticate(b'Bearer ' + TOKENS['A'][1])
                repeated = await gate.call(fresh, 'optic_session_info')
                assert repeated == session  # no implicit session creation/renewal
                state.control('revoke', 'A')
                try:
                    await gate.call(fresh, 'optic_fixture_read')
                except ForwardError:
                    pass
                else:
                    raise AssertionError('revoked principal admitted')
                assert await gate.stop()
                assert not gate.pending
    print('PASS: real Rust fixed fixture reads, stable session after dummy rotation, owner/stale/revoked denial and clean gate drain.')
    print('Local stdio integration only; no HTTP/FS host measurement or actual Rust session revoke claim.')


if __name__ == '__main__':
    if len(sys.argv) != 2:
        raise SystemExit('one fixed test binary is required')
    asyncio.run(run(sys.argv[1]))
