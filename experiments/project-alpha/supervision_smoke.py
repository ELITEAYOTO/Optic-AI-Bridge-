"""Native core supervision proof; generated workspace, no HTTP or real project."""
import asyncio
import base64
from pathlib import Path
import sys
import tempfile

from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client
from mcp.shared.exceptions import MCPError

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / 'real-link-admission'))
from real_smoke import structured


async def require_inactive(session, name, arguments):
    try:
        reply = await session.call_tool(name, arguments)
    except MCPError as error:
        assert error.message == 'optic.session_inactive', error.message
    else:
        assert reply.is_error, 'inactive authority was accepted'


async def exercise(binary, supervised):
    with tempfile.TemporaryDirectory(prefix='optic-supervisor-proof-') as directory:
        root = Path(directory)
        (root / 'public.txt').write_text('PUBLIC_SUPERVISION_FIXTURE\n', encoding='utf-8')
        arguments = [str(root), '--session-lifetime-seconds', '5']
        if supervised:
            arguments.append('--supervise-session')
        async with stdio_client(StdioServerParameters(command=str(binary), args=arguments)) as (read, write):
            async with ClientSession(read, write) as session:
                async with asyncio.timeout(30):
                    await session.initialize()
                    surface = {tool.name for tool in (await session.list_tools()).tools}
                    assert 'session_renew' not in surface and 'session_supervise' not in surface
                    before = structured(await session.call_tool('session_info', {}))
                    assert before['capabilities'] == ['file_read', 'file_search'], before
                    # Unknown remote methods cannot opt into local supervision.
                    try:
                        attempt = await session.call_tool('session_renew', {})
                    except MCPError:
                        pass
                    else:
                        assert attempt.is_error
                    # No traffic/heartbeat is required for this operator-owned timer.
                    await asyncio.sleep(6.5)
                    if not supervised:
                        await require_inactive(session, 'session_info', {})
                        return
                    after = structured(await session.call_tool('session_info', {}))
                    assert after['expires_at_monotonic_ms'] > before['expires_at_monotonic_ms']
                    assert {key: value for key, value in before.items() if key != 'expires_at_monotonic_ms'} == {
                        key: value for key, value in after.items() if key != 'expires_at_monotonic_ms'}
                    value = structured(await session.call_tool('fs_read', {'path': 'public.txt'}))
                    assert base64.b64decode(value['data']) == b'PUBLIC_SUPERVISION_FIXTURE\n'
                    cancelled = await session.call_tool('session_cancel', {})
                    assert not cancelled.is_error
                    # Cross a further renewal tick: cancel cannot be undone.
                    await asyncio.sleep(2.1)
                    for name, args in (('session_info', {}), ('fs_read', {'path': 'public.txt'})):
                        await require_inactive(session, name, args)


async def main(binary):
    binary = Path(binary).resolve(strict=True)
    for supervised in (False, True):
        await exercise(binary, supervised)
    print('SUPERVISION PASS: default expiry, explicit local renewal without traffic, unchanged grants, no renewal tool, revoke remains terminal.')


if __name__ == '__main__':
    asyncio.run(main(sys.argv[1]))
