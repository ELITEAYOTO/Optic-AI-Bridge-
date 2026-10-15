"""Real core JSON permission switches, using only disposable local inputs."""
import asyncio
import json
from pathlib import Path
import subprocess
import sys
import tempfile

from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client
from mcp.shared.exceptions import MCPError

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / 'real-link-admission'))
from real_smoke import structured


async def require_denied(session, tool, arguments):
    try:
        result = await session.call_tool(tool, arguments)
    except MCPError as error:
        assert error.message == 'optic.policy_denied', error.message
    else:
        assert result.is_error


async def exercise(binary):
    with tempfile.TemporaryDirectory(prefix='optic-permissions-sdk-') as directory:
        base = Path(directory)
        root, state = base / 'project', base / 'journal'
        root.mkdir()
        state.mkdir()
        (root / 'sentinel.txt').write_bytes(b'private-to-disabled-file-tools')
        config = base / 'operator.json'
        policy = {'version': 1, 'policy_epoch': 7, 'workspace': str(root),
                  'permissions': {'files': {'create_directories': sys.platform == 'win32'}},
                  'mutation': {'write_scopes': ['all'], 'delete_scopes': ['all'],
                               'directory_scopes': ['all'], 'state_dir': str(state)}}
        config.write_text(json.dumps(policy), encoding='utf-8')
        async with stdio_client(StdioServerParameters(command=str(binary), args=['--config', str(config)])) as (read, write):
            async with ClientSession(read, write) as session:
                await session.initialize()
                info = structured(await session.call_tool('session_info', {}))
                assert info['capabilities'] == (['directory_create'] if sys.platform == 'win32' else []), info
                assert info['policy_epoch'] == 7
                names = {item.name for item in (await session.list_tools()).tools}
                assert not {'fs_write', 'fs_apply_patch', 'fs_delete'} & names
                await require_denied(session, 'fs_read', {'path': 'sentinel.txt'})
                await require_denied(session, 'fs_list', {})
                if sys.platform == 'win32':
                    made = structured(await session.call_tool('fs_create_directory', {'path': 'modules', 'expected': 'absent'}))
                    assert made['created'] and (root / 'modules').is_dir()
                structured(await session.call_tool('session_cancel', {}))
        # The same policy inside writable sources must not be accepted as authority.
        inside = root / 'operator.json'
        inside.write_text(json.dumps(policy), encoding='utf-8')
        refused = subprocess.run([str(binary), '--config', str(inside)], input=b'',
                                 capture_output=True, timeout=10)
        assert refused.returncode != 0 and b'outside the writable workspace' in refused.stderr
        assert not refused.stdout
        # With the section omitted, the pre-existing explicitly selected read
        # baseline remains compatible. No hidden mutation scopes are added.
        config.write_text(json.dumps({'version': 1, 'policy_epoch': 8, 'workspace': str(root)}), encoding='utf-8')
        async with stdio_client(StdioServerParameters(command=str(binary), args=['--config', str(config)])) as (read, write):
            async with ClientSession(read, write) as session:
                await session.initialize()
                info = structured(await session.call_tool('session_info', {}))
                assert info['capabilities'] == ['file_read', 'file_search'], info
                result = structured(await session.call_tool('fs_read', {'path': 'sentinel.txt'}))
                assert result['eof']
    print('PERMISSIONS PASS: explicit booleans reduce real core capabilities/leases, directory-only operation, protected policy source and compatible legacy reads.')


if __name__ == '__main__':
    asyncio.run(exercise(Path(sys.argv[1]).resolve(strict=True)))
