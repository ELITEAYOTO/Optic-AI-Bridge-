"""Native typed mkdir proof on disposable roots; no user project or shell."""
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


async def denied(session, arguments, code=None):
    try:
        result = await session.call_tool('fs_create_directory', arguments)
    except MCPError as error:
        if code is not None:
            assert error.message == code, error.message
    else:
        assert result.is_error, 'unexpected directory admission'


async def exercise(binary):
    if sys.platform != 'win32':
        return
    with tempfile.TemporaryDirectory(prefix='optic-directory-sdk-') as temporary:
        base = Path(temporary)
        root, state = base / 'project', base / 'journal'
        root.mkdir()
        state.mkdir()
        (root / 'src').mkdir()
        common = [str(root), '--mutation-state-dir', str(state), '--allow-write-scope', 'prefix:src']
        # File write permission never silently grants folder creation.
        async with stdio_client(StdioServerParameters(command=str(binary), args=common)) as (read, write):
            async with ClientSession(read, write) as session:
                await session.initialize()
                assert 'fs_create_directory' not in {item.name for item in (await session.list_tools()).tools}
        arguments = common + ['--allow-directory-create-scope', 'prefix:src']
        async with stdio_client(StdioServerParameters(command=str(binary), args=arguments)) as (read, write):
            async with ClientSession(read, write) as session:
                await session.initialize()
                assert 'fs_create_directory' in {item.name for item in (await session.list_tools()).tools}
                await denied(session, {'path': 'src/components'})  # Absence must be explicit.
                await denied(session, {'path': 'src/components', 'expected': 'present'})
                await denied(session, {'path': '../escape', 'expected': 'absent'}, 'optic.invalid_workspace_path')
                await denied(session, {'path': 'outside', 'expected': 'absent'}, 'optic.policy_denied')
                await denied(session, {'path': 'src/missing/child', 'expected': 'absent'})
                assert not (root / 'src/missing').exists()
                result = structured(await session.call_tool('fs_create_directory', {'path': 'src/components', 'expected': 'absent'}))
                assert result['created'] and result['path'] == 'src/components'
                await denied(session, {'path': 'src/components', 'expected': 'absent'}, 'optic.precondition_failed')
                created = structured(await session.call_tool('fs_write', {
                    'path': 'src/components/example.js', 'expected': {'kind': 'absent'},
                    'content_base64': base64.b64encode(b'export const value = 42;\n').decode()}))
                assert created['path'] == 'src/components/example.js'
                assert (root / 'src/components/example.js').read_bytes() == b'export const value = 42;\n'
                structured(await session.call_tool('session_cancel', {}))
                await denied(session, {'path': 'src/after-cancel', 'expected': 'absent'}, 'optic.session_inactive')
                assert not (root / 'src/after-cancel').exists()
    print('DIRECTORY PASS: separate grant, absent precondition, scoped existing parent, nested versioned file write, conflicts and revoked admission.')


if __name__ == '__main__':
    asyncio.run(exercise(Path(sys.argv[1]).resolve(strict=True)))
